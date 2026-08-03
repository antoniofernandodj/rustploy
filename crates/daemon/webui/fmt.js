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
