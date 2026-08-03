// screens/new_service.js — form simplificado de criação de serviço. NÃO é o
// wizard de blueprints do client iced (catálogo de bancos/brokers/templates,
// new_service.gv) — isso fica para uma fase própria. Aqui só as duas origens
// mais diretas: repositório Git (clona + builda) ou imagem pronta de
// registry.
document.addEventListener("alpine:init", () => {
  Alpine.data("newService", () => ({
    get store() {
      return Alpine.store("app");
    },
    sourceKind: "git", // "git" | "registry"
    name: "",
    gitUrl: "",
    gitBranch: "main",
    registryImage: "",
    port: 80,
    domain: "",
    error: "",
    submitting: false,

    async submit() {
      this.error = "";
      if (!this.name.trim()) {
        this.error = "nome obrigatório";
        return;
      }
      let source;
      if (this.sourceKind === "git") {
        if (!this.gitUrl.trim()) {
          this.error = "URL do repositório obrigatória";
          return;
        }
        source = {
          Git: {
            url: this.gitUrl.trim(),
            branch: this.gitBranch.trim() || "main",
            root_path: "",
            watch_paths: [],
            submodules: false,
            dockerfile_path: "Dockerfile",
            build_context: ".",
            build_stage: null,
            credentials: null,
            username: null,
            provider_id: null,
          },
        };
      } else {
        if (!this.registryImage.trim()) {
          this.error = "imagem obrigatória";
          return;
        }
        source = { Registry: { image: this.registryImage.trim() } };
      }

      this.submitting = true;
      const r = await this.store.createService(
        this.name,
        this.store.selectedProjectId,
        source,
        this.port,
        this.domain
      );
      this.submitting = false;
      if (!r.ok) {
        this.error = r.error;
        return;
      }
      this.name = "";
      this.gitUrl = "";
      this.registryImage = "";
      this.domain = "";
    },

    cancel() {
      this.store.nav("project");
    },
  }));
});
