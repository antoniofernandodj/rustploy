// fmt.js — timestamps, durações e paleta de estado. Porta de
// crates/rustploy-gui/views/scripts/fmt/time.luau e fmt/util.luau (só as
// partes usadas pelo dashboard nesta fase).

/** epoch (ms) de um RFC3339 ("...Z" ou offset). */
function toEpochMs(iso) {
  if (typeof iso !== "string") return null;
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? null : ms;
}

/** "HH:MM:SS" local. */
export function timeHms(iso) {
  const ms = toEpochMs(iso);
  if (ms === null) return "";
  const d = new Date(ms);
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** "dd/mm HH:MM:SS" local. */
export function dateDmHms(iso) {
  const ms = toEpochMs(iso);
  if (ms === null) return "";
  const d = new Date(ms);
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}:${pad(d.getSeconds())}`;
}

/** Ns ou Mm Ns. */
export function fmtSecs(secs) {
  const n = Math.floor(Number(secs) || 0);
  const m = Math.floor(n / 60);
  const s = n % 60;
  return m > 0 ? `${m}m ${s}s` : `${s}s`;
}

/** dd hh mm / hh mm / mm ss, o maior campo não-zero primeiro. */
export function fmtUptime(secs) {
  const n = Math.floor(Number(secs) || 0);
  const d = Math.floor(n / 86400);
  const h = Math.floor((n % 86400) / 3600);
  const m = Math.floor((n % 3600) / 60);
  const s = n % 60;
  if (d > 0) return `${d}d ${h}h ${m}m`;
  if (h > 0) return `${h}h ${m}m`;
  return `${m}m ${s}s`;
}

/** Duração de um deployment (finished_at ou agora) − started_at. */
export function fmtDuration(dep) {
  const start = toEpochMs(dep.started_at);
  if (start === null) return "0s";
  const finish = dep.finished_at ? toEpochMs(dep.finished_at) : Date.now();
  const secs = Math.max(0, Math.floor(((finish ?? Date.now()) - start) / 1000));
  return fmtSecs(secs);
}

/** DeployState → (rótulo, kind semântico p/ .state_<kind>). */
export function stateLabelKind(state) {
  if (state === "Live") return ["LIVE", "ok"];
  if (state === "Stopped") return ["STOPPED", "muted"];
  if (state === "Failed") return ["FAILED", "bad"];
  return ["BUILDING", "info"];
}

/** ServiceStatus (string ou `{Error: "..."}`) → (rótulo, kind). Porta de
 * fmt/util.luau::status_label_color/status_kind. */
export function serviceStatusLabelKind(status) {
  if (typeof status === "object" && status?.Error !== undefined) return ["Error", "bad"];
  switch (status) {
    case "Running":
      return ["Running", "ok"];
    case "Deploying":
      return ["Deploying", "info"];
    case "Queued":
      return ["Na fila", "muted"];
    case "Degraded":
      return ["Degraded", "warn"];
    case "Stopping":
    case "Stopped":
      return ["Stopped", "muted"];
    default:
      return ["Error", "bad"];
  }
}

/** Tamanho de bytes legível ("—" para 0/ausente). */
export function fmtBytes(b) {
  const n = Number(b) || 0;
  if (n === 0) return "—";
  const KB = 1024,
    MB = KB * 1024,
    GB = MB * 1024;
  if (n >= GB) return (n / GB).toFixed(1) + " GB";
  if (n >= MB) return (n / MB).toFixed(0) + " MB";
  return (n / KB).toFixed(0) + " KB";
}

/** Resumo curto da origem de um serviço (ServiceSource externally-tagged). */
export function sourceSummary(source) {
  if (!source || typeof source !== "object") return "—";
  if (source.Registry) return source.Registry.image || "—";
  if (source.Git) return `${source.Git.url} @ ${source.Git.branch}`;
  if (source.Archive) return source.Archive.original_filename || source.Archive.archive_id || "zip enviado";
  if (source.Compose) return "docker-compose";
  return "—";
}

/** `<secret:NOME>` ou `secret:NOME` → "NOME" (ou null se não for referência).
 * Porta de helpers.luau::parse_secret_ref. */
export function parseSecretRef(v) {
  const s = (v || "").trim();
  const m = s.match(/^<secret:(.+)>$/) || s.match(/^secret:(.+)$/);
  if (!m) return null;
  const name = m[1].trim();
  return name || null;
}

/** env_vars + env_comments → texto `.env` (KEY=VALUE, secrets como
 * `<secret:NOME>`, comentários `# ...` na posição ancorada por `before_key`).
 * Porta literal de fmt/service_detail.luau::env_dotenv_with_comments. */
export function dotenvFromVars(vars, comments) {
  const activeVars = vars || [];
  const activeComments = comments || [];
  const lines = [];
  for (const v of activeVars) {
    for (const c of activeComments) {
      if (c.before_key === v.key) lines.push(c.text);
    }
    const val = v.value?.Secret !== undefined ? `<secret:${v.value.Secret}>` : v.value?.Plain || "";
    lines.push(`${v.key}=${val}`);
  }
  for (const c of activeComments) {
    if (c.before_key === null || c.before_key === undefined) lines.push(c.text);
  }
  return lines.join("\n");
}

/** Remove sequências de escape ANSI (cor/cursor/erase) de uma linha de log.
 * Porta literal de fmt/util.luau::strip_ansi. Sem isso, um logger colorido
 * (chalk/pino-pretty/etc.) manda bytes de controle que o HTML não interpreta
 * — em vez de cor, viram glifos de caixa (░/□) e o texto quebra visualmente
 * (linhas empilhando por cima umas das outras). */
export function stripAnsi(s) {
  let str = s || "";
  // CSI: ESC [ params byte-final(letra) — cobre SGR (cor), erase (K/J), cursor.
  str = str.replace(/\x1b\[[\d;?]*[a-zA-Z]/g, "");
  // OSC: ESC ] … BEL.
  str = str.replace(/\x1b\][^\x07]*\x07/g, "");
  // Qualquer ESC + 1 byte remanescente (ESC c, ESC =, …).
  str = str.replace(/\x1b./g, "");
  // CR/NUL soltos.
  str = str.replace(/[\r\0]/g, "");
  return str;
}

/** Texto `.env` → { vars, comments } (env_vars/env_comments do ServiceSpec/
 * Project). Porta literal de handlers/services.luau::parse_dotenv — linhas
 * `# ...` acumulam e ancoram (`before_key`) na próxima `KEY=VALUE` real;
 * sobras no fim viram comentários soltos (`before_key: null`). */
export function parseDotenv(text) {
  const vars = [];
  const comments = [];
  let pending = [];
  for (const raw of (text || "").split("\n")) {
    const l = raw.trim();
    if (l === "") continue;
    if (l.startsWith("#")) {
      pending.push(l);
      continue;
    }
    const eq = l.indexOf("=");
    if (eq < 0) continue;
    const key = l.slice(0, eq).trim();
    if (!key) continue;
    const v = l.slice(eq + 1).trim();
    for (const c of pending) comments.push({ text: c, before_key: key });
    pending = [];
    const secret = parseSecretRef(v);
    vars.push({ key, value: secret ? { Secret: secret } : { Plain: v } });
  }
  for (const c of pending) comments.push({ text: c, before_key: null });
  return { vars, comments };
}

// ── Connection tab: URLs de conexão por tipo de serviço/banco ────────────
// Porta de fmt/service_detail.luau (safe_name/internal_url/external_url +
// os helpers internos de credenciais/esquema/percent-encode).

/** Normaliza um nome de serviço para `[a-z0-9_]`, mesmo algoritmo de
 * `crate::normalize_name` (Rust) / `fmt/service_detail.luau::safe_name`. */
export function safeName(name) {
  let out = "";
  let lastDash = true;
  for (const ch of name || "") {
    if (/[a-zA-Z0-9]/.test(ch)) {
      out += ch.toLowerCase();
      lastDash = false;
    } else if (!lastDash) {
      out += "_";
      lastDash = true;
    }
  }
  return out.replace(/^_+/, "").replace(/_+$/, "");
}

function internalScheme(dbKind) {
  const k = (dbKind || "").toLowerCase();
  if (k === "postgres" || k === "postgresql") return "postgresql";
  if (k === "mysql" || k === "mariadb") return "mysql";
  if (k === "redis") return "redis";
  if (k === "mongodb" || k === "mongo") return "mongodb";
  if (k === "rabbitmq") return "amqp";
  if (k === "nats") return "nats";
  return null; // kafka / serviço comum: passthrough sem esquema
}

/** URL de conexão dentro da rede Docker do daemon (`rp_<safe>:<porta>`, com
 * esquema por tipo de banco). */
export function internalUrl(dbKind, safe, port) {
  const host = `rp_${safe}:${port}`;
  const scheme = internalScheme(dbKind);
  return scheme ? `${scheme}://${host}` : host;
}

function envPlain(vars, key) {
  const v = (vars || []).find((e) => e.key === key);
  return v?.value?.Plain || null;
}

/** (database, user, password) lidos das env vars conhecidas do banco. */
function dbCredentials(dbKind, vars) {
  const k = (dbKind || "").toLowerCase();
  if (k === "postgres" || k === "postgresql") {
    return [envPlain(vars, "POSTGRES_DB"), envPlain(vars, "POSTGRES_USER"), envPlain(vars, "POSTGRES_PASSWORD")];
  }
  if (k === "mysql" || k === "mariadb") {
    return [envPlain(vars, "MYSQL_DATABASE"), envPlain(vars, "MYSQL_USER"), envPlain(vars, "MYSQL_PASSWORD")];
  }
  if (k === "mongodb" || k === "mongo") {
    return [null, envPlain(vars, "MONGO_INITDB_ROOT_USERNAME"), envPlain(vars, "MONGO_INITDB_ROOT_PASSWORD")];
  }
  if (k === "redis") return [null, null, envPlain(vars, "REDIS_PASSWORD")];
  if (k === "rabbitmq") return [null, envPlain(vars, "RABBITMQ_DEFAULT_USER"), envPlain(vars, "RABBITMQ_DEFAULT_PASS")];
  return [null, null, null];
}

function withDbCredentials(base, database, user, password) {
  let url = base;
  if (database) url += `/${database}`;
  const params = [];
  if (user) params.push(`user=${user}`);
  if (password) params.push(`password=${password}`);
  if (params.length) url += `?${params.join("&")}`;
  return url;
}

function pct(s) {
  return encodeURIComponent(s).replace(/[!'()*]/g, (c) => "%" + c.charCodeAt(0).toString(16).toUpperCase());
}

function userinfo(user, password) {
  const u = user ? pct(user) : "";
  const p = password ? pct(password) : "";
  if (!u && !p) return "";
  return p ? `${u}:${p}@` : `${u}@`;
}

function externalScheme(k) {
  if (k === "mysql") return "mysql";
  if (k === "mariadb") return "mariadb";
  if (k === "redis") return "redis";
  if (k === "mongodb" || k === "mongo") return "mongodb";
  if (k === "rabbitmq") return "amqp";
  if (k === "nats") return "nats";
  return null;
}

function dbConnectionUrl(dbKind, host, port, database, user, password) {
  const k = (dbKind || "").toLowerCase();
  const hp = `${host}:${port}`;
  if (k === "postgres" || k === "postgresql") {
    return "jdbc:" + withDbCredentials(`postgresql://${hp}`, database, user, password);
  }
  const scheme = externalScheme(k);
  if (!scheme) return hp;
  let url = `${scheme}://${userinfo(user, password)}${hp}`;
  if (database) url += `/${database}`;
  else if (k === "mongodb" || k === "mongo") url += "/";
  if ((k === "mongodb" || k === "mongo") && user) url += "?authSource=admin";
  return url;
}

function urlHost(apiUrl) {
  const u = (apiUrl || "").replace(/\s/g, "");
  if (!u) return null;
  const m = u.match(/^[a-zA-Z][\w+.-]*:\/\/([^:/]+)/);
  return m ? m[1] : u.match(/^([^:/]+)/)?.[1] || null;
}

/** URL de conexão externa: domínio HTTP tem prioridade; sem domínio, cai
 * pro passthrough TCP (host_port) com a URL idiomática do banco. */
export function externalUrl(domain, tls, hostPort, dbKind, apiUrl, envVars) {
  const [database, user, password] = dbCredentials(dbKind, envVars);
  if (domain && domain.trim()) {
    const clean = domain.replace(/\/+$/, "");
    return `${tls ? "https" : "http"}://${clean}`;
  }
  if (hostPort) {
    const host = urlHost(apiUrl) || "<host>";
    return dbConnectionUrl(dbKind, host, String(hostPort), database, user, password);
  }
  return "—";
}
