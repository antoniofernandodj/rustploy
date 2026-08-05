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

/** "dd/mm HH:MM" local (sem segundos — usado em listas Docker/Registry). */
export function dateDmHm(iso) {
  const ms = toEpochMs(iso);
  if (ms === null) return "";
  const d = new Date(ms);
  const pad = (n) => String(n).padStart(2, "0");
  return `${pad(d.getDate())}/${pad(d.getMonth() + 1)} ${pad(d.getHours())}:${pad(d.getMinutes())}`;
}

/** `term` já em minúsculas; casa se algum campo (string) contém `term`
 * (substring, case-insensitive). Porta de fmt/util.luau::matches. */
function matchesTerm(term, fields) {
  if (!term) return true;
  return fields.some((f) => typeof f === "string" && f.toLowerCase().includes(term));
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
  if (state === "PreDeployCheck") return ["PRÉ-DEPLOY CHECK", "info"];
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

/** env_vars + env_comments → linhas pra exibição na lista normal (fora do
 * editor `.env` bruto), comentários `# ...` intercalados na posição ancorada
 * por `before_key`. Cada linha de comentário ganha uma `key` sintética
 * (`__cN`) pra servir de `:key` do `x-for` — comentários não têm chave
 * própria. Porta de fmt/service_detail.luau::env_json_with_comments. */
export function envRowsWithComments(vars, comments) {
  const activeVars = vars || [];
  const activeComments = comments || [];
  const rows = [];
  for (const v of activeVars) {
    activeComments.forEach((c, i) => {
      if (c.before_key === v.key) rows.push({ key: `__c${i}`, isComment: true, text: c.text });
    });
    rows.push({
      key: v.key,
      isComment: false,
      value: v.value?.Plain ?? (v.value?.Secret ? `<secret:${v.value.Secret}>` : ""),
      isSecret: !!v.value?.Secret,
    });
  }
  activeComments.forEach((c, i) => {
    if (c.before_key === null || c.before_key === undefined) {
      rows.push({ key: `__c${i}`, isComment: true, text: c.text });
    }
  });
  return rows;
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

// ── Deploy Engine / Monitoring / Ingress ──────────────────────────────────
// Porta de fmt/dashboard.luau (ingress/host_ports/monitoring/eng_*) e
// fmt/util.luau::domain_routes/pair_list — mesmas fórmulas, sem a truncagem
// por coluna (Util.ellipsis/col_budgets), que só existe no glacier por causa
// da janela redimensionável; aqui o CSS já cuida do overflow.

/** `msg.services` (`[{project_name, service}]`) → `[{svc, proj}]`. */
export function pairList(services) {
  return (services || []).map((e) => ({ svc: e.service, proj: e.project_name || "" }));
}

/** `spec.domains` se houver, senão o legado `domain`/`tls_enabled`. Porta
 * literal de fmt/util.luau::domain_routes — ver memória multi_domain_routes:
 * é o helper canônico, não reimplementar ad-hoc. */
export function domainRoutes(spec) {
  if (spec.domains && spec.domains.length > 0) {
    return spec.domains.map((r) => ({ domain: r.domain, port: r.port ?? null, tls: r.tls === true }));
  }
  if (spec.domain && spec.domain.trim()) {
    return [{ domain: spec.domain, port: null, tls: spec.tls_enabled === true }];
  }
  return [];
}

/** Ingress: uma linha por rota de domínio (não filtrado pela busca). */
export function ingressRows(pairs) {
  const rows = [];
  for (const p of pairs) {
    const spec = p.svc.spec;
    for (const r of domainRoutes(spec)) {
      if (r.domain && r.domain.trim()) {
        const cport = r.port ?? spec.port;
        rows.push({
          domain: r.domain,
          url: `${r.tls ? "https" : "http"}://${r.domain}`,
          service: spec.name,
          project: p.proj,
          upstream: `:${cport}`,
          tls: r.tls ? "TLS" : "—",
        });
      }
    }
  }
  return rows;
}

/** Portas TCP de host: uma linha por serviço com `host_port` configurado. */
export function hostPortRows(pairs) {
  const rows = [];
  for (const p of pairs) {
    const spec = p.svc.spec;
    if (spec.host_port != null) {
      rows.push({
        service: spec.name,
        project: p.proj,
        hostPort: spec.host_port,
        containerPort: spec.port,
      });
    }
  }
  rows.sort((a, b) => a.hostPort - b.hostPort);
  return rows;
}

/** Monitoring: uma linha por serviço COM métricas vivas (`metricsById[id]`
 * só existe depois do primeiro evento `ContainerMetrics`). */
export function monitoringRows(pairs, metricsById) {
  const rows = [];
  for (const p of pairs) {
    const m = metricsById && metricsById[p.svc.id];
    if (m) {
      rows.push({
        name: p.svc.spec.name,
        project: p.proj,
        cpu: `${(m.cpu_percent || 0).toFixed(1)}%`,
        mem: fmtBytes(m.mem_used_bytes),
        rx: fmtBytes(m.net_rx_bytes),
        tx: fmtBytes(m.net_tx_bytes),
      });
    }
  }
  return rows;
}

/** Barra de progresso em blocos (`█`/`░`). Porta literal de
 * fmt/time.luau::progress_bar. */
export function progressBar(percent, width = 12) {
  const p = Number(percent) || 0;
  let filled = Math.floor((p * width) / 100);
  if (filled > width) filled = width;
  if (filled < 0) filled = 0;
  return "█".repeat(filled) + "░".repeat(width - filled);
}

/** Deploy Engine: "Executando agora". */
export function engActiveRows(active) {
  return (active || []).map((info) => {
    const [label, kind] = stateLabelKind(info.state);
    return {
      service: info.service_name,
      project: info.project_name,
      stateLabel: label,
      stateKind: kind,
      percent: `${info.percent}%`,
      bar: progressBar(info.percent, 12),
      total: fmtSecs(info.elapsed_secs),
      phase: fmtSecs(info.current_state_secs),
      serviceId: info.service_id,
    };
  });
}

/** Deploy Engine: "Na fila" (o primeiro é o próximo a rodar). */
export function engQueuedRows(queued) {
  return (queued || []).map((info, i) => ({
    deploymentId: info.deployment_id,
    pos: i + 1,
    service: info.service_name,
    project: info.project_name,
  }));
}

// ── Docker (host-wide: containers/imagens/volumes/networks) ──────────────
// Porta de fmt/dashboard.luau::docker_containers/docker_images/docker_volumes/
// docker_networks — mesmas fórmulas, sem a truncagem por coluna (decisão já
// tomada na Fase 5: o CSS cuida do overflow na web).

/** Containers do host (rodando + parados). `canRemove` só nos parados (o
 * Docker recusa `rm` de um rodando sem force). `owner` mostra projeto/serviço
 * quando atribuído, senão "rustploy" (managed) ou "—". */
export function dockerContainerRows(list, term) {
  const t = (term || "").toLowerCase();
  const sorted = [...(list || [])].sort((a, b) => (a.name || "").localeCompare(b.name || ""));
  const rows = [];
  for (const c of sorted) {
    if (!matchesTerm(t, [c.name, c.image, c.state, c.project || "", c.service || ""])) continue;
    const stateLabel = containerStateLabel(c.state);
    let owner;
    if (c.project && c.service) owner = `${c.project} / ${c.service}`;
    else if (c.managed) owner = "rustploy";
    else owner = "—";
    const image = (c.image || "—").replace(/^sha256:/, "");
    rows.push({
      idFull: c.id || "",
      id: (c.id || "").slice(0, 12),
      name: c.name || "—",
      image,
      owner,
      stateLabel: stateLabel[0],
      stateKind: stateLabel[1],
      canRemove: containerStopped(c.state),
    });
  }
  return rows;
}

/** Rótulo/kind de um estado bruto do Docker ("running","exited",...). Porta
 * literal de fmt/util.luau::container_state. */
function containerStateLabel(state) {
  const s = (state || "").toLowerCase();
  if (s === "running") return ["running", "ok"];
  if (s === "restarting") return ["restarting", "info"];
  if (s === "paused") return ["paused", "warn"];
  if (s === "created") return ["created", "info"];
  if (s === "dead") return ["dead", "bad"];
  return [s || "exited", "muted"];
}

/** Porta literal de fmt/util.luau::container_stopped — só os parados podem
 * ser removidos (o Docker recusa `rm` de um rodando sem force). */
function containerStopped(state) {
  const s = (state || "").toLowerCase();
  return s !== "running" && s !== "restarting" && s !== "paused";
}

export function dockerImageRows(list, term) {
  const t = (term || "").toLowerCase();
  const sorted = [...(list || [])].sort((a, b) => (a.tags || []).join(",").localeCompare((b.tags || []).join(",")));
  const rows = [];
  for (const img of sorted) {
    const tagStr = (img.tags || []).join(" ");
    if (!matchesTerm(t, [tagStr, img.project || "", img.service || ""])) continue;
    const inUse = (img.containers || 0) > 0;
    const id = (img.id || "").replace(/^sha256:/, "");
    rows.push({
      id: id.slice(0, 12),
      idFull: id,
      tags: (img.tags || []).length ? img.tags.join(", ") : "<none>",
      size: fmtBytes(img.size_bytes),
      created: dateDmHm(img.created),
      project: img.project || "—",
      service: img.service || "—",
      owner: `${img.project || "—"} / ${img.service || "—"}`,
      inUse,
      inUseLabel: inUse ? "EM USO" : "SEM USO",
      inUseKind: inUse ? "ok" : "muted",
    });
  }
  return rows;
}

export function dockerVolumeRows(list, term) {
  const t = (term || "").toLowerCase();
  const sorted = [...(list || [])].sort((a, b) => (a.name || "").localeCompare(b.name || ""));
  const rows = [];
  for (const v of sorted) {
    if (!matchesTerm(t, [v.name, v.driver])) continue;
    rows.push({
      name: v.name,
      nameFull: v.name,
      driver: v.driver,
      mountpoint: v.mountpoint,
      size: v.size_bytes != null && v.size_bytes >= 0 ? fmtBytes(v.size_bytes) : "—",
      inUse: !!v.in_use,
      inUseLabel: v.in_use ? "EM USO" : "SEM USO",
      inUseKind: v.in_use ? "ok" : "muted",
    });
  }
  return rows;
}

export function dockerNetworkRows(list, term) {
  const t = (term || "").toLowerCase();
  const sorted = [...(list || [])].sort((a, b) => (a.name || "").localeCompare(b.name || ""));
  const rows = [];
  for (const n of sorted) {
    if (!matchesTerm(t, [n.name, n.project || ""])) continue;
    rows.push({
      name: n.name,
      idFull: n.id || "",
      driver: n.driver,
      scope: n.scope,
      project: n.project || "—",
      containers: n.container_count || 0,
      inUse: !!n.in_use,
      inUseLabel: n.in_use ? "EM USO" : "SEM USO",
      inUseKind: n.in_use ? "ok" : "muted",
    });
  }
  return rows;
}

// ── Registry OCI embutido ──────────────────────────────────────────────────
// Porta de fmt/registry.luau.

/** Lista de repositórios (filtrada pela busca global). */
export function registryRepoRows(list, term) {
  const t = (term || "").toLowerCase();
  const rows = [];
  for (const r of list || []) {
    if (!matchesTerm(t, [r.name])) continue;
    rows.push({
      name: r.name,
      tagCount: r.tag_count || 0,
      size: fmtBytes(r.size_bytes || 0),
      created: dateDmHm(r.created_at),
    });
  }
  return rows;
}

/** Tags de UM repositório (sem filtro — lista pequena, buscada sob demanda). */
export function registryTagRows(list) {
  return (list || []).map((t) => ({
    tag: t.tag,
    digest: (t.digest || "").slice(0, 12),
    digestFull: t.digest,
    size: fmtBytes(t.size_bytes || 0),
    updated: dateDmHm(t.updated_at),
  }));
}

/** Tokens de acesso Basic auth (sem filtro — lista pequena). */
export function registryTokenRows(list) {
  return (list || []).map((t) => ({
    name: t.name,
    scope: t.scope,
    created: dateDmHm(t.created_at),
    lastUsed: t.last_used_at ? dateDmHm(t.last_used_at) : "nunca",
  }));
}

// ── Schedules (jobs one-shot via docker-compose) ──────────────────────────
// Porta de fmt/jobs.luau.

const WEEKDAYS = ["seg", "ter", "qua", "qui", "sex", "sáb", "dom"];

/** Resumo textual de uma `Recurrence?` (nil = só manual). Porta literal de
 * fmt/jobs.luau::recurrence_label. */
export function recurrenceLabel(r) {
  if (r == null) return "manual";
  if (r.IntervalHours != null) return `a cada ${r.IntervalHours}h`;
  if (r.Daily) {
    const pad = (n) => String(n).padStart(2, "0");
    return `diariamente às ${pad(r.Daily.hour)}:${pad(r.Daily.minute)}`;
  }
  if (r.Weekly) {
    const pad = (n) => String(n).padStart(2, "0");
    const wd = WEEKDAYS[((r.Weekly.weekday % 7) + 7) % 7] || "?";
    return `semanalmente ${wd} às ${pad(r.Weekly.hour)}:${pad(r.Weekly.minute)}`;
  }
  return "—";
}

/** Rótulo/kind da última execução de um job. `run == null` (nunca rodou) /
 * `run.success == null` (rodando agora) / `true`/`false` (ok/falhou). Porta
 * literal de fmt/jobs.luau::run_status. */
export function jobRunStateLabel(run) {
  if (run == null) return ["nunca rodou", "muted"];
  if (run.success == null) return ["rodando…", "warn"];
  return run.success ? ["ok", "ok"] : ["falhou", "bad"];
}

/** Tela global "Schedules": uma linha por job, de todos os projetos. */
export function jobSummaryRows(list, term) {
  const t = (term || "").toLowerCase();
  const rows = [];
  for (const s of list || []) {
    const job = s.job;
    if (!matchesTerm(t, [job.name, s.project_name, s.trigger_service_name || ""])) continue;
    const [lastRunLabel, lastRunKind] = jobRunStateLabel(s.last_run);
    rows.push({
      id: job.id,
      name: job.name,
      project: s.project_name,
      owner: s.trigger_service_name ? `${s.project_name} / ${s.trigger_service_name}` : s.project_name,
      recurrence: recurrenceLabel(job.recurrence),
      enabled: job.enabled,
      enabledLabel: job.enabled ? "Pausar" : "Ativar",
      lastRunLabel,
      lastRunKind,
      // Usado pra desativar/trocar o botão "Rodar agora" enquanto o job_run
      // em voo não termina (mesma condição de lastRunKind === "warn").
      running: lastRunKind === "warn",
      lastRunId: s.last_run ? s.last_run.id : "",
      nextRunAt: job.next_run_at ? dateDmHm(job.next_run_at) : "—",
    });
  }
  return rows;
}

// ── Settings (Web Server / Git / Infra as Code) ───────────────────────────

/** Redirect URI do OAuth (`{base}/oauth/{gitea|github}/callback`). Porta
 * literal de helpers.luau::oauth_redirect_uri. */
export function oauthRedirectUri(base, kind) {
  const seg = kind === "github" ? "github" : "gitea";
  const d = (base || "").trim().replace(/\/+$/, "");
  if (d === "") return `<URL pública do daemon indisponível>/oauth/${seg}/callback`;
  return `${d}/oauth/${seg}/callback`;
}

function gitKindLabel(kind) {
  if (kind === "Github") return "GitHub";
  if (kind === "Gitea") return "Gitea";
  return kind || "Git";
}

/** Porta literal de fmt/git.luau::git_providers. */
export function gitProviderRows(list) {
  return (list || []).map((p) => {
    const login = p.account?.login;
    const connected = login != null;
    return {
      id: p.id,
      name: p.name,
      kind: gitKindLabel(p.kind),
      display: connected ? `${p.name || ""} (@${login})` : `${p.name || ""} — não conectado`,
      baseUrl: p.base_url,
      authMode: p.auth_mode === "OAuth" ? "OAuth2" : "PAT",
      account: connected ? `@${login}` : "(pendente — autorize no navegador)",
      connected,
    };
  });
}

/** Deploy Engine: "Histórico 24h". */
export function engRecentRows(recent) {
  return (recent || []).map((info) => {
    const [label, kind] = stateLabelKind(info.state);
    let icon = "○";
    if (info.state === "Live") icon = "✓";
    else if (info.state === "Failed") icon = "✕";
    return {
      icon,
      service: info.service_name,
      project: info.project_name,
      stateLabel: label,
      stateKind: kind,
      duration: fmtSecs(info.elapsed_secs),
      start: timeHms(info.started_at),
    };
  });
}
