// fmt.js — timestamps, durações e paleta de estado. Porta de
// crates/rustploy-gui/views/scripts/fmt/time.luau e fmt/util.luau (só as
// partes usadas pelo dashboard nesta fase).

/** epoch (ms) de um RFC3339 ("...Z" ou offset). */
function toEpochMs(iso) {
  if (typeof iso !== "string") return null;
  const ms = Date.parse(iso);
  return Number.isNaN(ms) ? null : ms;
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
