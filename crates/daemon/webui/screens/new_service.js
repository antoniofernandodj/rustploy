// screens/new_service.js — wizard "Novo serviço", porta de new_service.gv
// (client iced): passo pick_type → passo específico por tipo. Application usa
// `ServiceCreate` direto (coleta a origem Git/registry já na criação — ver
// app.js::createServiceDirect); Compose/Database/Broker/Template usam
// `Command::WizardCreate` (o daemon monta o compose/env vars certos a partir
// dos catálogos de `Command::WizardCatalog`).
document.addEventListener("alpine:init", () => {
  Alpine.data("newService", () => ({
    get store() {
      return Alpine.store("app");
    },
    step: "pick_type",
    error: "",
    submitting: false,

    catalog: { dbs: [], brokers: [], templates: [] },
    catalogLoaded: false,
    async ensureCatalog() {
      if (this.catalogLoaded) return;
      this.catalog = await this.store.fetchWizardCatalog("");
      this.catalogLoaded = true;
    },

    // ── Application ────────────────────────────────────────────────
    appName: "",
    sourceKind: "git",
    gitUrl: "",
    gitBranch: "main",
    registryImage: "",
    appPort: 80,
    appDomain: "",

    // ── Compose Stack ─────────────────────────────────────────────
    composeName: "",
    composeText: "services:\n  app:\n    image: nginx:latest\n    ports:\n      - \"80:80\"\n",
    composePort: 80,
    composeDomain: "",

    // ── Database ──────────────────────────────────────────────────
    selectedDb: null,
    dbServiceName: "",
    dbName: "",
    dbUser: "",
    dbPassword: "",
    dbRootPassword: "",
    dbImage: "",
    dbUseReplica: false,

    // ── Broker ────────────────────────────────────────────────────
    selectedBroker: null,
    brokerServiceName: "",
    brokerUser: "",
    brokerPassword: "",
    brokerImage: "",

    // ── Template ──────────────────────────────────────────────────
    templateSearch: "",
    selectedTemplate: null,
    templateServiceName: "",
    templateValues: [],
    get filteredTemplates() {
      const t = this.templateSearch.trim().toLowerCase();
      if (!t) return this.catalog.templates;
      return this.catalog.templates.filter(
        (x) => x.name.toLowerCase().includes(t) || x.description.toLowerCase().includes(t)
      );
    },

    // ── Navegação por passos ─────────────────────────────────────────
    gotoType() {
      this.error = "";
      this.step = "pick_type";
    },
    gotoApp() {
      this.error = "";
      this.step = "app_form";
    },
    gotoCompose() {
      this.error = "";
      this.step = "compose_form";
    },
    async gotoDb() {
      this.error = "";
      await this.ensureCatalog();
      this.step = "pick_db";
    },
    pickDb(db) {
      this.selectedDb = db;
      this.dbUser = db.user;
      this.dbImage = db.image;
      this.dbName = "";
      this.dbPassword = "";
      this.dbRootPassword = "";
      this.dbUseReplica = false;
      this.dbServiceName = "";
      this.step = "db_form";
    },
    async gotoBroker() {
      this.error = "";
      await this.ensureCatalog();
      this.step = "pick_broker";
    },
    pickBroker(b) {
      this.selectedBroker = b;
      this.brokerUser = b.user;
      this.brokerImage = b.image;
      this.brokerPassword = "";
      this.brokerServiceName = "";
      this.step = "broker_form";
    },
    async gotoTemplate() {
      this.error = "";
      await this.ensureCatalog();
      this.step = "pick_template";
    },
    pickTemplate(t) {
      this.selectedTemplate = t;
      this.templateValues = t.vars.map(() => "");
      this.templateServiceName = "";
      this.step = "template_form";
    },

    cancel() {
      this.store.nav("project");
    },

    // ── Submissões ────────────────────────────────────────────────────
    baseReq(kind, id) {
      return {
        kind,
        id,
        project_id: this.store.selectedProjectId,
        name: "",
        app_name: "",
        db_name: "",
        user: "",
        password: "",
        root_password: "",
        image: "",
        use_replica: false,
        template_values: [],
        expose_external: false,
      };
    },

    async submitApp() {
      this.error = "";
      if (!this.appName.trim()) {
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
      const r = await this.store.createServiceDirect(this.appName, source, this.appPort, this.appDomain);
      this.submitting = false;
      if (!r.ok) this.error = r.error;
    },

    async submitCompose() {
      this.error = "";
      if (!this.composeName.trim()) {
        this.error = "nome obrigatório";
        return;
      }
      if (!this.composeText.trim()) {
        this.error = "compose YAML obrigatório";
        return;
      }
      // WizardCreateReq (kind="compose") não carrega o YAML — o wizard iced
      // cria o serviço com compose VAZIO e deixa editar depois na aba
      // General. Preenchemos já na criação via ServiceCreate direto, como
      // fazemos com Application (mais direto pro usuário).
      const source = { Compose: { content: this.composeText } };
      this.submitting = true;
      const r = await this.store.createServiceDirect(this.composeName, source, this.composePort, this.composeDomain);
      this.submitting = false;
      if (!r.ok) this.error = r.error;
    },

    async submitDb() {
      this.error = "";
      const db = this.selectedDb;
      if (db.has_db_name && !this.dbName.trim()) {
        this.error = "nome do banco obrigatório";
        return;
      }
      if (db.has_user && !this.dbUser.trim()) {
        this.error = "usuário obrigatório";
        return;
      }
      if (!this.dbPassword.trim()) {
        this.error = "senha obrigatória";
        return;
      }
      const req = this.baseReq("database", db.id);
      req.name = this.dbServiceName;
      req.db_name = this.dbName;
      req.user = this.dbUser;
      req.password = this.dbPassword;
      req.root_password = this.dbRootPassword;
      req.image = this.dbImage;
      req.use_replica = this.dbUseReplica;
      this.submitting = true;
      const r = await this.store.wizardCreate(req);
      this.submitting = false;
      if (!r.ok) this.error = r.error;
    },

    async submitBroker() {
      this.error = "";
      const b = this.selectedBroker;
      if (b.has_user && !this.brokerUser.trim()) {
        this.error = "usuário obrigatório";
        return;
      }
      const req = this.baseReq("broker", b.id);
      req.name = this.brokerServiceName;
      req.user = this.brokerUser;
      req.password = this.brokerPassword;
      req.image = this.brokerImage;
      this.submitting = true;
      const r = await this.store.wizardCreate(req);
      this.submitting = false;
      if (!r.ok) this.error = r.error;
    },

    async submitTemplate() {
      this.error = "";
      const req = this.baseReq("template", this.selectedTemplate.id);
      req.name = this.templateServiceName;
      req.template_values = this.templateValues;
      this.submitting = true;
      const r = await this.store.wizardCreate(req);
      this.submitting = false;
      if (!r.ok) this.error = r.error;
    },
  }));
});
