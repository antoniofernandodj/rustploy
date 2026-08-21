// screens/project_detail.js — projeto aberto (view=project_services no
// client iced): sub-abas Serviços/Variáveis/Secrets/Jobs. Porta de
// fmt.service_rows, o cabeçalho editável e as sub-abas env/secrets/jobs de
// shell.gv + handlers/projects.luau (aba "Jobs" filtra `snap.jobs` pelo
// projeto aberto — mesma lógica de stream.luau::update_open_project). O
// wizard "novo job" acionado pelo botão desta aba é o mesmo modal global do
// store (app.js), compartilhado com a tela "Schedules".
import {
  serviceStatusLabelKind,
  dotenvFromVars,
  parseDotenv,
  envRowsWithComments,
  jobSummaryRows,
  fmtBytes,
} from "../fmt.js";

/** Container "primário" de um serviço pra exibir no card: o live, senão o
 * primeiro em execução, senão o primeiro da lista. `extra` é "+N" quando há
 * mais de um container. Porta de fmt/dashboard.luau::primary_container. */
function primaryContainer(svc) {
  const list = svc.containers || [];
  if (list.length === 0) return { name: "—", id: "", extra: "" };
  let chosen = list[0];
  for (const c of list) {
    if (svc.live_container_id && c.id === svc.live_container_id) {
      chosen = c;
      break;
    }
    if (c.state === "running" && chosen.state !== "running") chosen = c;
  }
  const extra = list.length > 1 ? `+${list.length - 1}` : "";
  return { name: chosen.name || "—", id: (chosen.id || "").slice(0, 12), extra };
}

document.addEventListener("alpine:init", () => {
  Alpine.data("projectDetail", () => ({
    get store() {
      return Alpine.store("app");
    },
    editing: false,
    editName: "",
    editDesc: "",
    projTab: "services", // "services" | "env" | "secrets" | "jobs"

    get project() {
      const s = this.store;
      return ((s.snap && s.snap.projects) || []).find((p) => p.id === s.selectedProjectId) || null;
    },

    get services() {
      const s = this.store;
      const pid = s.selectedProjectId;
      const services = (s.snap && s.snap.services) || [];
      const metricsById = s.metricsById || {};
      return services
        .filter((e) => e.service.spec.project_id === pid)
        .map((e) => {
          const svc = e.service;
          const [label, kind] = serviceStatusLabelKind(svc.status);
          const m = metricsById[svc.id];
          const container = primaryContainer(svc);
          return {
            id: svc.id,
            name: svc.spec.name,
            port: svc.spec.port,
            statusLabel: label,
            statusKind: kind,
            cpu: m ? `${(m.cpu_percent || 0).toFixed(1)}%` : "—",
            mem: m ? fmtBytes(m.mem_used_bytes) : "—",
            containerName: container.name,
            containerId: container.id,
            containerExtra: container.extra,
          };
        });
    },

    get canDelete() {
      return this.services.length === 0;
    },

    // ── Jobs do projeto ──────────────────────────────────────────────
    get jobs() {
      const pid = this.store.selectedProjectId;
      const list = (this.store.snap?.jobs || []).filter((s) => s.job.project_id === pid);
      return jobSummaryRows(list, "", this.store.jobsInflight);
    },

    startEdit() {
      const p = this.project;
      if (!p) return;
      this.editName = p.name;
      this.editDesc = p.description || "";
      this.editing = true;
    },
    cancelEdit() {
      this.editing = false;
    },
    async saveEdit() {
      const r = await this.store.updateProject(this.store.selectedProjectId, this.editName, this.editDesc);
      if (r.ok) this.editing = false;
    },

    // ── Variáveis do projeto ─────────────────────────────────────────
    newEnvKey: "",
    newEnvValue: "",
    envTextOpen: false,
    envText: "",

    get envVars() {
      const p = this.project;
      if (!p) return [];
      return envRowsWithComments(p.env_vars, p.env_comments);
    },

    async addEnvVar() {
      if (!this.newEnvKey.trim()) return;
      const p = this.project;
      const vars = (p.env_vars || []).filter((e) => e.key !== this.newEnvKey.trim());
      vars.push({ key: this.newEnvKey.trim(), value: { Plain: this.newEnvValue } });
      const r = await this.store.saveProjectEnv(vars, p.env_comments || []);
      if (r.ok) {
        this.newEnvKey = "";
        this.newEnvValue = "";
      }
    },
    async delEnvVar(key) {
      const p = this.project;
      const vars = (p.env_vars || []).filter((e) => e.key !== key);
      const comments = (p.env_comments || []).filter((c) => c.before_key !== key);
      await this.store.saveProjectEnv(vars, comments);
    },

    openEnvText() {
      const p = this.project;
      this.envText = dotenvFromVars(p?.env_vars, p?.env_comments);
      this.envTextOpen = true;
    },
    closeEnvText() {
      this.envTextOpen = false;
    },
    async saveEnvText() {
      const { vars, comments } = parseDotenv(this.envText);
      const r = await this.store.saveProjectEnv(vars, comments);
      if (r.ok) this.envTextOpen = false;
    },

    // ── Secrets do projeto ────────────────────────────────────────────
    newSecretName: "",
    newSecretValue: "",

    get secrets() {
      const p = this.project;
      return (p?.secrets || []).map((name) => ({ name }));
    },

    async submitSecret() {
      const r = await this.store.addSecret(this.newSecretName, this.newSecretValue);
      if (r.ok) {
        this.newSecretName = "";
        this.newSecretValue = "";
      }
    },
  }));
});
