// screens/project_detail.js — projeto aberto (view=project_services no
// client iced): sub-abas Serviços/Variáveis/Secrets (Jobs fica para depois).
// Porta de fmt.service_rows, o cabeçalho editável e as sub-abas env/secrets
// de shell.gv + handlers/projects.luau.
import { serviceStatusLabelKind, dotenvFromVars, parseDotenv, envRowsWithComments } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("projectDetail", () => ({
    get store() {
      return Alpine.store("app");
    },
    editing: false,
    editName: "",
    editDesc: "",
    projTab: "services", // "services" | "env" | "secrets"

    get project() {
      const s = this.store;
      return ((s.snap && s.snap.projects) || []).find((p) => p.id === s.selectedProjectId) || null;
    },

    get services() {
      const s = this.store;
      const pid = s.selectedProjectId;
      const services = (s.snap && s.snap.services) || [];
      return services
        .filter((e) => e.service.spec.project_id === pid)
        .map((e) => {
          const svc = e.service;
          const [label, kind] = serviceStatusLabelKind(svc.status);
          // CPU/mem ao vivo dependem do evento ContainerMetrics do bus
          // (Command::MetricsSubscribe) — ainda não consumido nesta fase
          // (fica para a tela Monitoring). Placeholder honesto por ora.
          return {
            id: svc.id,
            name: svc.spec.name,
            port: svc.spec.port,
            statusLabel: label,
            statusKind: kind,
            cpu: "—",
            mem: "—",
          };
        });
    },

    get canDelete() {
      return this.services.length === 0;
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
