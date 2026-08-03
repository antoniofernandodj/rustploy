// screens/project_detail.js — projeto aberto (view=project_services no
// client iced, só a sub-aba "Serviços" nesta fase — Variáveis/Secrets/Jobs
// do projeto ficam para depois). Porta de fmt.service_rows + o cabeçalho
// editável de shell.gv.
import { serviceStatusLabelKind } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("projectDetail", () => ({
    get store() {
      return Alpine.store("app");
    },
    editing: false,
    editName: "",
    editDesc: "",

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
  }));
});
