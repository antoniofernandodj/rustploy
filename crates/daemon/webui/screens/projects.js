// screens/projects.js — tela "Projects": grid de cards + criar/editar/
// remover. Porta de fmt.project_rows (crates/rustploy-gui/views/scripts/
// fmt/dashboard.luau) e handlers/projects.luau (create/edit/delete), sem o
// grid em N-colunas do client iced (aqui é CSS grid nativo, ver .grid em
// app.css) nem a janela separada de criação (o browser não tem multi-janela
// — o form fica inline, mesma tela).
document.addEventListener("alpine:init", () => {
  Alpine.data("projects", () => ({
    get store() {
      return Alpine.store("app");
    },
    showNewForm: false,
    newName: "",
    newDesc: "",
    newError: "",
    editingId: null,
    editName: "",
    editDesc: "",

    get rows() {
      const s = this.store;
      const services = (s.snap && s.snap.services) || [];
      return ((s.snap && s.snap.projects) || []).map((p) => {
        const svcs = services.filter((e) => e.service.spec.project_id === p.id);
        return {
          id: p.id,
          name: p.name,
          description: p.description || "",
          serviceCount: svcs.length,
          runningCount: svcs.filter((e) => e.service.status === "Running").length,
          canDelete: svcs.length === 0,
        };
      });
    },

    async submitNew() {
      this.newError = "";
      const r = await this.store.createProject(this.newName, this.newDesc);
      if (r.ok) {
        this.newName = "";
        this.newDesc = "";
        this.showNewForm = false;
      } else {
        this.newError = r.error;
      }
    },

    startEdit(row) {
      this.editingId = row.id;
      this.editName = row.name;
      this.editDesc = row.description;
    },
    cancelEdit() {
      this.editingId = null;
    },
    async saveEdit() {
      const r = await this.store.updateProject(this.editingId, this.editName, this.editDesc);
      if (r.ok) this.editingId = null;
    },
  }));
});
