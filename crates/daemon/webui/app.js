// app.js — único <script type="module"> carregado por index.html. Orquestra
// a ordem de boot do Alpine "na mão" (em vez do CDN clássico com `defer`):
// um `<script defer>` do Alpine e módulos ES separados entram na mesma fila
// de execução em ordem, mas plugins/CDNs alternativos podem correr fora de
// ordem — e um `Alpine.store`/`Alpine.data` registrado DEPOIS que o Alpine já
// disparou `alpine:init` nunca é visto (x-show/x-data leem `undefined` e a
// tela fica em branco). Aqui a ordem é garantida pelos próprios `import`
// (sempre resolvidos antes do corpo do módulo que os declara):
//   1. screens/*.js rodam primeiro — só REGISTRAM listeners de `alpine:init`.
//   2. Alpine é importado como módulo (não via `<script>` solto).
//   3. este módulo registra o `Alpine.store('app', …)` (a "ctx" global).
//   4. só então `Alpine.start()` — dispara `alpine:init`, todos os
//      listeners acima já registrados.
import "./screens/login.js";
import "./screens/dashboard.js";
import "./screens/deploy_engine.js";
import "./screens/monitoring.js";
import "./screens/ingress.js";
import "./screens/docker.js";
import "./screens/schedules.js";
import "./screens/settings.js";
import "./screens/projects.js";
import "./screens/project_detail.js";
import "./screens/new_service.js";
import "./screens/service_detail.js";
import Alpine from "https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/module.esm.js";
import { Api } from "./net/api.js";
import { openStream } from "./net/sse.js";
import {
  fmtUptime,
  fmtBytes,
  stripAnsi,
  oauthRedirectUri,
  timeHms,
  parseDotenv,
  dotenvFromVars,
} from "./fmt.js";

window.Alpine = Alpine;

const PREFS_KEY = "rustploy.prefs";

function loadPrefs() {
  try {
    return JSON.parse(localStorage.getItem(PREFS_KEY) || "{}");
  } catch {
    return {};
  }
}

function savePrefs(p) {
  localStorage.setItem(PREFS_KEY, JSON.stringify(p));
}

document.addEventListener("alpine:init", () => {
  const prefs = loadPrefs();

  Alpine.store("app", {
    // ── Sessão / navegação (≈ ctx.screen/ctx.view do glacier) ────────────
    screen: "login",
    view: "deployments",
    connected: false,
    statusLine: "pronto para conectar",
    error: "",
    erroUrl: "",

    // ── Formulário de login ───────────────────────────────────────────
    url: prefs.rememberUrl && prefs.url ? prefs.url : "",
    token: prefs.rememberToken && prefs.token ? prefs.token : "",
    rememberUrl: !!prefs.rememberUrl,
    rememberToken: !!prefs.rememberToken,

    // ── Sessão ativa ─────────────────────────────────────────────────
    api: null,
    stream: null,
    search: "",

    // ── Dados do snapshot (≈ State.snap) ────────────────────────────
    snap: null,
    dataLoading: true,
    daemonVersion: "",
    daemonUptime: "…",
    servicesLabel: "…",
    deploymentsMsg: "",

    // ── Eventos ao vivo do bus (≈ State.metrics_by_id / ctx.sys_*) ─────
    // Só chegam por /api/events kind="bus" — o snapshot periódico (2s) não
    // carrega métricas por container nem do host (ver onStreamEvent).
    metricsById: {}, // service_id -> ContainerMetricsPoint
    sysCpu: "—",
    sysMem: "—",
    sysDisk: "—",
    sysLoad: "—",
    engineMsg: "",

    // ── Docker / Registry (aba Docker) ──────────────────────────────────
    dockerTab: "containers",
    dockerMsg: "",
    onlyUsedImages: false,
    onlyUsedVolumes: false,
    onlyUsedNetworks: false,
    pruneAllImages: false,
    pruneAllVolumes: false,
    registrySelectedRepo: "",
    registryTags: [],
    registryTagsLoading: false,
    registryTokens: [],
    registryMsg: "",

    // ── Schedules (jobs one-shot) ────────────────────────────────────────
    jobsById: {}, // id -> Job cru (porta de State.jobs_by_id, precisa do
    // record completo pra reenviar em JobUpdate — só `enabled` muda)
    jobMsg: "",
    jobLogLines: [],
    jobLogStream: null,

    // Wizard "novo job" (3 passos: projeto → serviço gatilho → form) — mora
    // no store (não num componente de tela) porque duas entradas abrem o
    // MESMO wizard: a tela global "Schedules" e a aba "Jobs" do projeto
    // (equivalente ao new_job_window do cliente iced, aberto por ambos
    // handlers/jobs.luau::open_new_job_window e a aba do projeto). Modal
    // renderizado uma vez em index.html, fora do escopo de qualquer tela.
    showNewJob: false,
    njobStep: "pick_project", // "pick_project" | "pick_service" | "form"
    // Modo edição: aberto por openEditJob(id) em vez do fluxo normal — pula
    // pro passo "form" (project_id/trigger_service_id não são editáveis via
    // JobUpdate, então não há passos 1/2). `null` = criando um job novo.
    njobEditId: null,
    njobEnabled: true,
    njobProjectId: "",
    njobProjectName: "",
    njobServiceId: "",
    njobServiceName: "",
    njobName: "",
    njobCompose: "",
    // Fonte do compose: "compose" (colado, default) | "git" (clona um repo a
    // cada execução, mesmo picker conta→repo→branch da aba Git de serviço —
    // ver njobSetSourceTab/njobGitProviderPick/njobGitRepoPick).
    njobSourceTab: "compose",
    njobGitProviderId: "",
    njobGitProviders: [],
    njobGitRepos: [],
    njobGitRepoFullName: "",
    njobGitBranches: [],
    njobGitBranch: "",
    njobComposePath: "docker-compose.yml",
    njobGitMsg: "",
    njobMainService: "",
    // Env vars próprias do job — maior precedência (por cima de projeto +
    // serviço gatilho). Blob .env colado, parseado em njobCreate.
    njobEnvText: "",
    njobKind: "manual", // "manual" | "interval" | "daily" | "weekly"
    njobHours: "6",
    njobHour: "3",
    njobMinute: "0",
    njobWeekday: "0",
    njobErr: "",
    njobSubmitting: false,

    // Modal de logs ao vivo de uma execução de job — mesma razão do wizard
    // acima (aberto tanto da tela Schedules quanto da aba Jobs do projeto).
    jobLogsFor: null,

    // ── Settings (Web Server / Git / Infra as Code) ─────────────────────
    settingsTab: "web",
    ssPublicBase: "",
    ssEmail: "",
    ssRegistryDomain: "",
    settingsMsg: "",
    gitProviders: [],
    gpKind: "gitea", // "gitea" | "github"
    gpMode: "oauth", // "oauth" | "pat"
    gpName: "",
    gpBaseUrl: "",
    gpClientId: "",
    gpClientSecret: "",
    gpPat: "",
    gpMsg: "",
    gpOauthUrl: "",
    iacYaml: "",
    iacDotenv: "",
    iacHasExport: false,
    iacExportMsg: "",
    iacImportYaml: "",
    iacImportDotenv: "",
    iacPrune: false,
    iacDeploy: false,
    iacHasMissing: false,
    iacMissingVars: "",
    iacHasReport: false,
    iacReportLines: [],
    iacImportMsg: "",

    get gpRedirect() {
      return oauthRedirectUri(this.ssPublicBase, this.gpKind);
    },

    // ── Navegação (≈ ctx.view do glacier) ───────────────────────────
    // "deployments" | "projects" | "project" | "service" | "new_service"
    view: "deployments",
    selectedProjectId: null,
    selectedServiceId: null,
    projectMsg: "",

    // ── Detalhe de serviço aberto ────────────────────────────────────
    serviceDetail: null, // Service (ServiceGet)
    serviceDeployments: [], // Vec<Deployment> (DeployHistory)
    serviceTab: "general",
    serviceMsg: "",
    serviceLoading: false,
    serviceLogLines: [],
    serviceLogStream: null,

    nav(view) {
      this.stopServiceLogs();
      this.view = view;
      if (view === "projects") this.projectMsg = "";
    },

    persistPrefs() {
      savePrefs({
        rememberUrl: this.rememberUrl,
        rememberToken: this.rememberToken,
        url: this.rememberUrl ? this.url : undefined,
        token: this.rememberToken ? this.token : undefined,
      });
    },

    normalizeUrl(raw) {
      const u = (raw || "").trim();
      if (!u) return null;
      const m = u.match(/^([a-zA-Z][\w+.-]*):\/\/(.*)$/);
      let scheme, rest;
      if (m) {
        scheme = m[1].toLowerCase();
        if (scheme !== "http" && scheme !== "https") return null;
        rest = m[2];
      } else {
        scheme = "https";
        rest = u;
      }
      rest = rest.split(/[/?#]/)[0];
      if (!rest) return null;
      return `${scheme}://${rest}`;
    },

    async connect() {
      const base = this.normalizeUrl(this.url);
      if (!base) {
        this.erroUrl = "URL inválida (ex.: https://rustploy.dominio.com)";
        return;
      }
      this.erroUrl = "";
      this.url = base;
      this.persistPrefs();
      this.statusLine = "conectando…";
      this.error = "";

      const client = new Api(base, this.token);
      const r = await client.rpc("DaemonStatus");
      if (!r.ok) {
        this.error = r.error;
        this.statusLine = "falha na conexão";
        this.connected = false;
        return;
      }

      this.api = client;
      this.connected = true;
      this.statusLine = "conectado";
      this.screen = "shell";
      this.dataLoading = true;
      await this.loadSettings();
      this.openStream();
    },

    disconnect() {
      this.stopServiceLogs();
      this.stream?.close();
      this.stream = null;
      this.api = null;
      this.snap = null;
      this.connected = false;
      this.screen = "login";
      this.statusLine = "desconectado";
      this.view = "deployments";
      this.selectedProjectId = null;
      this.selectedServiceId = null;
      this.serviceDetail = null;
    },

    openStream() {
      this.stream?.close();
      this.stream = openStream(this.api.baseUrl, this.api.token, "/api/events", {
        onEvent: (kind, data) => this.onStreamEvent(data),
        onError: (msg) => {
          this.statusLine = "stream: " + msg;
        },
        onClose: () => {
          this.stream = null;
          if (this.connected) {
            this.statusLine = "conexão encerrada";
            this.connected = false;
            this.screen = "login";
          }
        },
      });
    },

    onStreamEvent(msg) {
      if (!msg || typeof msg !== "object") return;
      if (msg.kind === "snapshot") this.applySnapshot(msg);
      else if (msg.kind === "bus") this.applyBusEvent(msg.event);
    },

    /** Um evento do bus (mesmo formato que stream.luau::apply_bus trata) —
     * unit variants chegam como string crua (ex. "DeployQueueChanged"). */
    applyBusEvent(ev) {
      if (ev === "DeployQueueChanged") {
        this.refreshNow();
        return;
      }
      if (!ev || typeof ev !== "object") return;
      if (ev.ContainerMetrics) {
        const p = ev.ContainerMetrics;
        if (p.service_id) this.metricsById[p.service_id] = p;
      } else if (ev.SystemMetrics) {
        const s = ev.SystemMetrics;
        this.sysCpu = `${(s.cpu_percent || 0).toFixed(0)}%`;
        this.sysMem = `${fmtBytes(s.mem_used_bytes)} / ${fmtBytes(s.mem_total_bytes)}`;
        this.sysDisk = `${fmtBytes(s.disk_used_bytes)} / ${fmtBytes(s.disk_total_bytes)}`;
        this.sysLoad = `${(s.load_avg_1 || 0).toFixed(2)} ${(s.load_avg_5 || 0).toFixed(2)} ${(s.load_avg_15 || 0).toFixed(2)}`;
      }
    },

    applySnapshot(msg) {
      this.snap = msg;
      const st = msg.status;
      if (st) {
        this.daemonVersion = st.version || "";
        this.daemonUptime = fmtUptime(st.uptime_secs);
        this.servicesLabel = `${st.services_running || 0}/${st.services_total || 0}`;
      }
      const byId = {};
      for (const s of msg.jobs || []) byId[s.job.id] = s.job;
      this.jobsById = byId;
      this.dataLoading = false;
    },

    searchChanged(v) {
      this.search = v || "";
    },

    async stopAll() {
      if (!confirm("Parar todos os serviços? Todos os serviços em execução serão parados agora.")) {
        return;
      }
      if (!this.api) return;
      this.statusLine = "parando todos…";
      const r = await this.api.rpcChecked("StopAllManaged");
      this.statusLine = r.ok ? "todos os serviços parados" : "erro: " + r.error;
    },

    async clearFinished() {
      if (!this.snap) return;
      const ids = (this.snap.deployments || [])
        .map((s) => s.deployment)
        .filter((d) => d.state === "Stopped" || d.state === "Failed")
        .map((d) => d.id);
      if (ids.length === 0) {
        this.deploymentsMsg = "nada para limpar";
        return;
      }
      if (
        !confirm(
          `Remove do histórico ${ids.length} deployment(s) em estado Stopped ou Failed, e seus build logs. Ação irreversível.`
        )
      ) {
        return;
      }
      this.deploymentsMsg = `removendo ${ids.length} deployment(s)…`;
      let removed = 0,
        failed = 0;
      for (const id of ids) {
        const r = await this.api.rpcChecked({ DeployDelete: { deployment_id: id } });
        if (r.ok) removed++;
        else failed++;
      }
      this.deploymentsMsg =
        failed === 0 ? `${removed} removido(s)` : `${removed} removido(s), ${failed} falharam`;
    },

    async refreshNow() {
      if (!this.api) return;
      const r = await this.api.rpc("Snapshot");
      if (r.ok && r.value && typeof r.value.Snapshot === "string") {
        try {
          this.applySnapshot(JSON.parse(r.value.Snapshot));
        } catch {
          /* snapshot malformado — ignora, o próximo tick de 2s corrige */
        }
      }
    },

    // ── Projects ─────────────────────────────────────────────────────

    async createProject(name, description) {
      if (!name || !name.trim()) return { ok: false, error: "nome obrigatório" };
      const r = await this.api.rpcChecked({
        ProjectCreate: { name: name.trim(), description: description?.trim() || null },
      });
      if (r.ok) await this.refreshNow();
      return r;
    },

    async updateProject(id, name, description) {
      const r = await this.api.rpcChecked({
        ProjectUpdate: { id, name: name.trim(), description: description?.trim() || null },
      });
      if (r.ok) await this.refreshNow();
      return r;
    },

    async deleteProject(id) {
      if (!confirm("Remover este projeto? Só funciona se não houver serviços nele.")) {
        return { ok: false, error: "cancelado" };
      }
      const r = await this.api.rpcChecked({ ProjectDelete: { id } });
      if (r.ok) {
        await this.refreshNow();
        this.nav("projects");
      }
      return r;
    },

    openProject(id) {
      this.selectedProjectId = id;
      this.projectMsg = "";
      this.nav("project");
    },

    async saveProjectEnv(envVars, envComments) {
      const r = await this.api.rpcChecked({
        ProjectEnvSet: {
          project_id: this.selectedProjectId,
          env_vars: envVars,
          env_comments: envComments,
        },
      });
      this.projectMsg = r.ok ? "variáveis salvas" : "erro: " + r.error;
      if (r.ok) await this.refreshNow();
      return r;
    },

    async addSecret(name, value) {
      if (!name.trim()) return { ok: false, error: "nome obrigatório" };
      const r = await this.api.rpcChecked({
        SecretSet: { project_id: this.selectedProjectId, name: name.trim(), value },
      });
      if (r.ok) await this.refreshNow();
      return r;
    },

    async deleteSecret(name) {
      if (!confirm(`Remover o secret "${name}"? Serviços que o referenciam vão falhar no próximo deploy.`)) {
        return;
      }
      const r = await this.api.rpcChecked({
        SecretDelete: { project_id: this.selectedProjectId, name },
      });
      if (r.ok) await this.refreshNow();
    },

    // ── Services ─────────────────────────────────────────────────────

    openNewService() {
      this.nav("new_service");
    },

    /** Catálogos do wizard (bancos/brokers/templates) — buscados uma vez ao
     * abrir a tela "Novo serviço" (ver screens/new_service.js). */
    async fetchWizardCatalog(search) {
      const r = await this.api.rpc({ WizardCatalog: { search: search || "" } });
      if (!r.ok || !r.value?.WizardCatalog) return { dbs: [], brokers: [], templates: [] };
      const c = r.value.WizardCatalog;
      try {
        return {
          dbs: JSON.parse(c.dbs),
          brokers: JSON.parse(c.brokers),
          templates: JSON.parse(c.templates),
        };
      } catch {
        return { dbs: [], brokers: [], templates: [] };
      }
    },

    /** `req` é o `WizardCreateReq` completo (ver screens/new_service.js) — o
     * daemon (shared::wizard::build_spec) monta o ServiceSpec certo conforme
     * `req.kind`. Usado por Compose/Database/Broker/Template: o backend gera
     * o compose e as env vars corretas (mesma lógica do client iced). */
    async wizardCreate(req) {
      const r = await this.api.rpcChecked({ WizardCreate: req });
      if (r.ok) {
        await this.refreshNow();
        const created = r.value?.Service;
        if (created) this.openService(created.id);
        else this.nav("project");
      }
      return r;
    },

    /** `source` já é o `ServiceSource` externally-tagged. Usado só pelo tipo
     * "Application" do wizard: diferente dos outros 4 tipos, o
     * `WizardCreateReq` não carrega URL de Git nem imagem de registry — o
     * wizard iced cria um serviço vazio (`Registry{image:""}`) e deixa o
     * usuário preencher depois na aba General. Aqui coletamos a origem já na
     * criação (mais direto), então vamos por `ServiceCreate` puro. */
    async createServiceDirect(name, source, port, domain) {
      const spec = {
        name: name.trim(),
        project_id: this.selectedProjectId,
        source,
        port: Number(port) || 80,
        host_port: null,
        domain: domain?.trim() || null,
        tls_enabled: false,
        env_vars: [],
        env_comments: [],
        volumes: [],
        healthcheck: {
          kind: "None",
          interval_secs: 5,
          timeout_secs: 3,
          retries: 10,
          start_period_secs: 5,
        },
        replicas: 1,
        resources: { cpu_shares: 0, mem_limit_bytes: 0 },
        run_command: null,
        run_args: [],
        db_kind: null,
        domains: [],
      };
      const r = await this.api.rpcChecked({ ServiceCreate: spec });
      if (r.ok) {
        await this.refreshNow();
        const created = r.value?.Service;
        if (created) this.openService(created.id);
        else this.nav("project");
      }
      return r;
    },

    async openService(id) {
      this.selectedServiceId = id;
      this.serviceTab = "general";
      this.serviceMsg = "";
      this.serviceDeployments = [];
      this.serviceLoading = true;
      this.nav("service");
      await this.fetchServiceDetail(id);
    },

    async fetchServiceDetail(id) {
      const r = await this.api.rpc({ ServiceGet: { id } });
      if (!r.ok || !r.value?.Service) {
        this.serviceMsg = r.ok ? "serviço não encontrado" : r.error;
        this.serviceLoading = false;
        return;
      }
      this.serviceDetail = r.value.Service;
      const rh = await this.api.rpc({ DeployHistory: { service_id: id, limit: 30 } });
      this.serviceDeployments = (rh.ok && rh.value?.Deployments) || [];
      this.serviceLoading = false;
    },

    async saveServiceSpec(spec, okMsg) {
      const id = this.selectedServiceId;
      const r = await this.api.rpcChecked({ ServiceUpdate: { id, spec } });
      if (r.ok) {
        this.serviceMsg = okMsg || "salvo";
        await this.fetchServiceDetail(id);
        await this.refreshNow();
      } else {
        this.serviceMsg = "erro: " + r.error;
      }
      return r;
    },

    async deleteService(id) {
      if (!confirm("Remover este serviço? Para o container e apaga o histórico. Ação irreversível.")) {
        return;
      }
      const r = await this.api.rpcChecked({ ServiceDelete: { id } });
      if (r.ok) {
        await this.refreshNow();
        this.nav("project");
      } else {
        this.serviceMsg = "erro ao remover: " + r.error;
      }
    },

    async deployStart() {
      const id = this.selectedServiceId;
      this.serviceMsg = "iniciando deploy…";
      const r = await this.api.rpcChecked({ DeployStart: { service_id: id } });
      this.serviceMsg = r.ok ? "deploy iniciado" : "erro: " + r.error;
      await this.fetchServiceDetail(id);
      await this.refreshNow();
    },

    async deployAbort(deploymentId) {
      const r = await this.api.rpcChecked({ DeployAbort: { deployment_id: deploymentId } });
      this.serviceMsg = r.ok ? "deploy cancelado" : "erro: " + r.error;
      await this.fetchServiceDetail(this.selectedServiceId);
    },

    async deployRollback() {
      if (!confirm("Reverter para o deploy anterior?")) return;
      const id = this.selectedServiceId;
      const r = await this.api.rpcChecked({ DeployRollback: { service_id: id } });
      this.serviceMsg = r.ok ? "rollback iniciado" : "erro: " + r.error;
      await this.fetchServiceDetail(id);
    },

    async deleteDeployment(deploymentId) {
      const r = await this.api.rpcChecked({ DeployDelete: { deployment_id: deploymentId } });
      if (r.ok) await this.fetchServiceDetail(this.selectedServiceId);
      return r;
    },

    // ── Deploy Engine (fila global) ─────────────────────────────────────
    // Porta de handlers/deploy_queue.luau — sem drag-and-drop (só glacier);
    // "↑ topo" e "✕" cobrem furar fila / desistir de um item.

    /** Cancela um deploy que ainda espera na fila — reaproveita DeployAbort
     * (o daemon já trata remoção da fila como caso do abort). */
    async queueCancel(deploymentId) {
      const r = await this.api.rpcChecked({ DeployAbort: { deployment_id: deploymentId } });
      this.engineMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    async queuePromote(deploymentId) {
      const r = await this.api.rpcChecked({ DeployQueuePromote: { deployment_id: deploymentId } });
      this.engineMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    async queueTogglePause() {
      const paused = !!this.snap?.engine?.paused;
      const r = await this.api.rpcChecked({ DeployQueuePause: { paused: !paused } });
      this.engineMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    // ── Docker (host-wide) ──────────────────────────────────────────────
    // Porta de handlers/docker.luau — cada prune/remove segue o mesmo padrão:
    // rpcChecked + mensagem em dockerMsg + refreshNow() pra refletir na hora
    // (o snapshot de 2s pegaria de qualquer jeito, mas assim fica imediato).

    /** Formata Response::PruneResult{count,reclaimed_bytes}; sem esse payload
     * (algum prune que devolve só Ok), mensagem genérica. */
    pruneResultMsg(r) {
      if (!r.ok) return "erro: " + r.error;
      const pr = r.value?.PruneResult;
      if (pr) return `removidos: ${pr.count} · ${fmtBytes(pr.reclaimed_bytes)} liberados`;
      return "limpeza concluída";
    },

    async dockerPruneContainers() {
      const r = await this.api.rpcChecked("PruneContainers");
      this.dockerMsg = this.pruneResultMsg(r);
      await this.refreshNow();
    },
    async dockerPruneImages() {
      const r = await this.api.rpcChecked({ PruneImages: { all: this.pruneAllImages } });
      this.dockerMsg = this.pruneResultMsg(r);
      await this.refreshNow();
    },
    async dockerPruneVolumes() {
      const r = await this.api.rpcChecked({ PruneVolumes: { all: this.pruneAllVolumes } });
      this.dockerMsg = this.pruneResultMsg(r);
      await this.refreshNow();
    },
    async dockerPruneNetworks() {
      const r = await this.api.rpcChecked("PruneNetworks");
      this.dockerMsg = this.pruneResultMsg(r);
      await this.refreshNow();
    },

    async dockerRemoveContainer(id) {
      const r = await this.api.rpcChecked({ RemoveContainer: { id } });
      this.dockerMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) await this.refreshNow();
    },
    async dockerRemoveImage(id) {
      const r = await this.api.rpcChecked({ RemoveImage: { id } });
      this.dockerMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) await this.refreshNow();
    },
    async dockerRemoveVolume(name) {
      const r = await this.api.rpcChecked({ RemoveVolume: { name } });
      this.dockerMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) await this.refreshNow();
    },
    async dockerRemoveNetwork(id) {
      const r = await this.api.rpcChecked({ RemoveNetwork: { id } });
      this.dockerMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) await this.refreshNow();
    },

    /** Troca a sub-aba Docker; ao entrar em "registry" busca os tokens (não
     * vêm no snapshot periódico, diferente de repos/containers/imagens). */
    async dockerSetTab(tab) {
      this.dockerTab = tab;
      if (tab === "registry") await this.registryRefreshTokens();
    },

    // ── Registry OCI embutido ────────────────────────────────────────────

    async registryOpenRepo(name) {
      this.registrySelectedRepo = name;
      this.registryTagsLoading = true;
      const r = await this.api.rpc({ RegistryTagList: { repo: name } });
      this.registryTags = (r.ok && r.value?.RegistryTags) || [];
      this.registryTagsLoading = false;
    },
    registryCloseRepo() {
      this.registrySelectedRepo = "";
      this.registryTags = [];
    },

    /** Sem rpcChecked de propósito (mesmo comportamento silencioso do Luau —
     * falha aqui não é acionável pelo usuário, só deixa a lista vazia). */
    async registryRefreshTokens() {
      const r = await this.api.rpc("RegistryTokenList");
      this.registryTokens = (r.ok && r.value?.RegistryTokens) || [];
    },

    async registryRmTag(tag) {
      const repo = this.registrySelectedRepo;
      const r = await this.api.rpcChecked({ RegistryTagDelete: { repo, tag } });
      this.registryMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) {
        await this.registryOpenRepo(repo);
        await this.refreshNow();
      }
    },
    async registryRmRepo(name) {
      const r = await this.api.rpcChecked({ RegistryRepoDelete: { repo: name } });
      this.registryMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) {
        if (this.registrySelectedRepo === name) this.registryCloseRepo();
        await this.refreshNow();
      }
    },
    async registryGc() {
      const r = await this.api.rpcChecked("RegistryGc");
      if (r.ok) {
        const gc = r.value?.RegistryGcResult;
        this.registryMsg = gc
          ? `GC: ${gc.blobs_removed} arquivo(s) removido(s) · ${fmtBytes(gc.bytes_freed)} liberados`
          : "GC concluído";
        await this.refreshNow();
      } else {
        this.registryMsg = "erro: " + r.error;
      }
    },
    async registryRmToken(name) {
      const r = await this.api.rpcChecked({ RegistryTokenRevoke: { name } });
      this.registryMsg = r.ok ? "" : "erro: " + r.error;
      if (r.ok) await this.registryRefreshTokens();
    },

    /** Devolve {ok, secret} pro modal de "novo token" — o segredo só existe
     * nesta resposta, nunca mais é recuperável depois. */
    async registryCreateToken(name, scope) {
      const r = await this.api.rpcChecked({ RegistryTokenCreate: { name, scope } });
      if (!r.ok) return { ok: false, error: r.error };
      await this.registryRefreshTokens();
      return { ok: true, secret: r.value.RegistryTokenCreated.secret };
    },

    // ── Schedules (jobs one-shot) ────────────────────────────────────────
    // Porta de handlers/jobs.luau.

    async jobRunNow(id) {
      const r = await this.api.rpcChecked({ JobRunNow: { id } });
      this.jobMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    /** Reenvia o Job inteiro (só `enabled` inverte) — o daemon não tem um
     * PATCH parcial; mesma limitação do cliente iced. */
    async jobToggle(id) {
      const job = this.jobsById[id];
      if (!job) {
        this.jobMsg = "job não encontrado no snapshot atual";
        return;
      }
      const r = await this.api.rpcChecked({
        JobUpdate: {
          id: job.id,
          name: job.name,
          compose: job.compose,
          git_source: job.git_source ?? null,
          main_service: job.main_service,
          env_vars: job.env_vars ?? [],
          env_comments: job.env_comments ?? [],
          enabled: !job.enabled,
          recurrence: job.recurrence,
        },
      });
      this.jobMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    async jobDelete(id) {
      if (!confirm("Remover este job? Ação irreversível.")) return;
      const r = await this.api.rpcChecked({ JobDelete: { id } });
      this.jobMsg = r.ok ? "" : "erro: " + r.error;
      await this.refreshNow();
    },

    /** `payload` = { project_id, trigger_service_id, name, compose,
     * main_service, recurrence }. */
    async jobCreate(payload) {
      const r = await this.api.rpcChecked({ JobCreate: payload });
      if (r.ok) await this.refreshNow();
      return r;
    },

    // ── Wizard "novo job" (ver campos no bloco de estado acima) ──────────
    // Porta de new_job_window.luau — mesmos passos e mesma validação.

    get njobProjects() {
      return this.snap?.projects || [];
    },
    /** Serviços do projeto escolhido — já em memória (snap.services), sem
     * re-fetch (diferente do Luau, que precisa pré-semear a janela isolada). */
    get njobServicesFiltered() {
      return (this.snap?.services || []).filter(
        (e) => e.service.spec.project_id === this.njobProjectId
      );
    },

    openNewJob() {
      this.showNewJob = true;
      this.njobStep = "pick_project";
      this.njobEditId = null;
      this.njobEnabled = true;
      this.njobProjectId = "";
      this.njobProjectName = "";
      this.njobServiceId = "";
      this.njobServiceName = "";
      this.njobName = "";
      this.njobCompose = "";
      this.njobSourceTab = "compose";
      this.njobGitProviderId = "";
      this.njobGitProviders = [];
      this.njobGitRepos = [];
      this.njobGitRepoFullName = "";
      this.njobGitBranches = [];
      this.njobGitBranch = "";
      this.njobComposePath = "docker-compose.yml";
      this.njobGitMsg = "";
      this.njobMainService = "";
      this.njobEnvText = "";
      this.njobKind = "manual";
      this.njobHours = "6";
      this.njobHour = "3";
      this.njobMinute = "0";
      this.njobWeekday = "0";
      this.njobErr = "";
    },
    closeNewJob() {
      this.showNewJob = false;
    },
    njobPickProject(id, name) {
      this.njobProjectId = id;
      this.njobProjectName = name;
      this.njobStep = "pick_service";
    },
    njobPickService(id, name) {
      this.njobServiceId = id;
      this.njobServiceName = name;
      this.njobStep = "form";
    },
    njobPickNoService() {
      this.njobServiceId = "";
      this.njobServiceName = "nenhum (autônomo)";
      this.njobStep = "form";
    },
    njobBack() {
      // Modo edição: não há passos 1/2 pra voltar (project_id/
      // trigger_service_id não são editáveis via JobUpdate) — fecha o modal.
      if (this.njobEditId) {
        this.closeNewJob();
        return;
      }
      if (this.njobStep === "form") this.njobStep = "pick_service";
      else if (this.njobStep === "pick_service") this.njobStep = "pick_project";
    },

    // ── Fonte do compose: aba "Compose" x aba "Git" (picker conta→repo→branch) ──
    // Porta de njob_source/njob_git_provider_pick/njob_git_repo_pick do
    // cliente iced (new_job_window.luau) — mesma resolução, sem a limitação
    // de janela isolada (aqui é só um fetch preguiçoso na primeira troca de aba).
    async njobSetSourceTab(kind) {
      this.njobSourceTab = kind;
      if (kind === "git" && this.njobGitProviders.length === 0) {
        this.njobGitMsg = "carregando contas…";
        const r = await this.api.rpc("GitProviderList");
        if (r.ok && r.value?.GitProviders) {
          this.njobGitProviders = r.value.GitProviders;
          this.njobGitMsg = "";
        } else {
          this.njobGitMsg = "erro ao listar contas conectadas";
        }
      }
    },
    async njobGitProviderPick(id) {
      this.njobGitProviderId = id || "";
      this.njobGitRepoFullName = "";
      this.njobGitRepos = [];
      this.njobGitBranches = [];
      if (!this.njobGitProviderId) return;
      this.njobGitMsg = "carregando repositórios…";
      const r = await this.api.rpc({ GitRepoList: { provider_id: this.njobGitProviderId } });
      if (r.ok && r.value?.GitRepos) {
        this.njobGitRepos = r.value.GitRepos;
        this.njobGitMsg = `${r.value.GitRepos.length} repositório(s)`;
      } else {
        this.njobGitMsg = "erro ao listar repositórios";
      }
    },
    async njobGitRepoPick(fullName) {
      if (!fullName) return;
      this.njobGitRepoFullName = fullName;
      const repo = this.njobGitRepos.find((r) => r.full_name === fullName);
      if (repo?.default_branch) this.njobGitBranch = repo.default_branch;
      this.njobGitBranches = [];
      if (!this.njobGitProviderId) return;
      this.njobGitMsg = "carregando branches…";
      const r = await this.api.rpc({
        GitBranchList: { provider_id: this.njobGitProviderId, repo_full_name: fullName },
      });
      if (r.ok && r.value?.GitBranches) {
        this.njobGitBranches = r.value.GitBranches;
        this.njobGitMsg = "";
      } else {
        this.njobGitMsg = "erro ao listar branches";
      }
    },

    /** Monta `recurrence` (Option<Recurrence>, externally-tagged) a partir
     * de njobKind. */
    buildNjobRecurrence() {
      if (this.njobKind === "interval") {
        return { IntervalHours: Math.max(1, Number(this.njobHours) || 1) };
      }
      if (this.njobKind === "daily") {
        return { Daily: { hour: Number(this.njobHour) || 0, minute: Number(this.njobMinute) || 0 } };
      }
      if (this.njobKind === "weekly") {
        return {
          Weekly: {
            weekday: Number(this.njobWeekday) || 0,
            hour: Number(this.njobHour) || 0,
            minute: Number(this.njobMinute) || 0,
          },
        };
      }
      return null;
    },

    async njobCreate() {
      if (!this.njobName.trim() || !this.njobMainService.trim()) {
        this.njobErr = "nome e main service são obrigatórios";
        return;
      }
      let compose = "";
      let gitSource = null;
      if (this.njobSourceTab === "git") {
        const branch = this.njobGitBranch.trim();
        if (!this.njobGitRepoFullName || !branch) {
          this.njobErr = "selecione repositório e branch";
          return;
        }
        const composePath = this.njobComposePath.trim() || "docker-compose.yml";
        const repo = this.njobGitRepos.find((r) => r.full_name === this.njobGitRepoFullName);
        if (!repo?.clone_url) {
          this.njobErr = "não foi possível resolver a URL do repositório — selecione de novo";
          return;
        }
        gitSource = {
          provider_id: this.njobGitProviderId || null,
          url: repo.clone_url,
          branch,
          username: null,
          credentials: null,
          compose_path: composePath,
        };
      } else {
        compose = this.njobCompose;
        if (!compose.trim()) {
          this.njobErr = "cole o docker-compose.yml do job";
          return;
        }
      }
      this.njobErr = "";
      this.njobSubmitting = true;
      const { vars: envVars, comments: envComments } = parseDotenv(this.njobEnvText);
      let r;
      if (this.njobEditId) {
        r = await this.api.rpcChecked({
          JobUpdate: {
            id: this.njobEditId,
            name: this.njobName.trim(),
            compose,
            git_source: gitSource,
            main_service: this.njobMainService.trim(),
            env_vars: envVars,
            env_comments: envComments,
            enabled: this.njobEnabled,
            recurrence: this.buildNjobRecurrence(),
          },
        });
        if (r.ok) await this.refreshNow();
      } else {
        r = await this.jobCreate({
          project_id: this.njobProjectId,
          // "" (não null) = job autônomo — Command::JobCreate::trigger_service_id
          // é String simples no protocolo (não Option<String>); o handler no
          // daemon é quem converte "" → None (job_create.rs).
          trigger_service_id: this.njobServiceId || "",
          name: this.njobName.trim(),
          compose,
          git_source: gitSource,
          main_service: this.njobMainService.trim(),
          env_vars: envVars,
          env_comments: envComments,
          recurrence: this.buildNjobRecurrence(),
        });
      }
      this.njobSubmitting = false;
      if (r.ok) this.closeNewJob();
      else this.njobErr = r.error;
    },

    /** Abre o mesmo modal do wizard, mas em modo edição: pula pro passo
     * "form" já preenchido com o job existente (project_id/trigger_service_id
     * não são editáveis via JobUpdate, então não há passos 1/2 aqui). Só
     * webui — sem a limitação de janela isolada do cliente iced, então
     * resolve a conta/repo/branch do git_source direto, sem pré-fetch. */
    async openEditJob(id) {
      const job = this.jobsById[id];
      if (!job) {
        this.jobMsg = "job não encontrado no snapshot atual";
        return;
      }
      this.showNewJob = true;
      this.njobEditId = id;
      this.njobStep = "form";
      this.njobErr = "";
      this.njobName = job.name || "";
      this.njobMainService = job.main_service || "";
      this.njobEnabled = !!job.enabled;
      this.njobEnvText = dotenvFromVars(job.env_vars, job.env_comments);

      const rec = job.recurrence;
      if (rec?.IntervalHours != null) {
        this.njobKind = "interval";
        this.njobHours = String(rec.IntervalHours);
      } else if (rec?.Daily) {
        this.njobKind = "daily";
        this.njobHour = String(rec.Daily.hour);
        this.njobMinute = String(rec.Daily.minute);
      } else if (rec?.Weekly) {
        this.njobKind = "weekly";
        this.njobWeekday = String(rec.Weekly.weekday);
        this.njobHour = String(rec.Weekly.hour);
        this.njobMinute = String(rec.Weekly.minute);
      } else {
        this.njobKind = "manual";
      }

      if (job.git_source) {
        this.njobSourceTab = "git";
        this.njobCompose = "";
        this.njobGitBranch = job.git_source.branch || "";
        this.njobComposePath = job.git_source.compose_path || "docker-compose.yml";
        this.njobGitProviderId = job.git_source.provider_id || "";
        if (this.njobGitProviderId) {
          const rp = await this.api.rpc("GitProviderList");
          if (rp.ok && rp.value?.GitProviders) this.njobGitProviders = rp.value.GitProviders;
          const rr = await this.api.rpc({ GitRepoList: { provider_id: this.njobGitProviderId } });
          if (rr.ok && rr.value?.GitRepos) {
            this.njobGitRepos = rr.value.GitRepos;
            const match = this.njobGitRepos.find((repo) => repo.clone_url === job.git_source.url);
            this.njobGitRepoFullName = match?.full_name || "";
            if (this.njobGitRepoFullName) {
              const rb = await this.api.rpc({
                GitBranchList: {
                  provider_id: this.njobGitProviderId,
                  repo_full_name: this.njobGitRepoFullName,
                },
              });
              if (rb.ok && rb.value?.GitBranches) this.njobGitBranches = rb.value.GitBranches;
            }
          }
        }
      } else {
        this.njobSourceTab = "compose";
        this.njobCompose = job.compose || "";
      }
    },

    openJobLogs(jobRunId) {
      this.jobLogsFor = jobRunId;
      this.startJobLogs(jobRunId);
    },
    closeJobLogs() {
      this.jobLogsFor = null;
      this.stopJobLogs();
    },

    /** Logs ao vivo de UMA execução de job — mesmo par seed+SSE de
     * startServiceLogs/stopServiceLogs, apontando pro endpoint dedicado de
     * job run (gêmeo do de serviço: mesmo framing bus_batch). */
    async startJobLogs(jobRunId) {
      this.stopJobLogs();
      const seed = await this.api.rpc({ GetJobLogs: { job_run_id: jobRunId } });
      this.jobLogLines = ((seed.ok && seed.value?.JobLogs) || []).map(this.cleanLogEntry);
      this.jobLogStream = openStream(
        this.api.baseUrl,
        this.api.token,
        `/api/jobs/runs/${jobRunId}/logs`,
        {
          onEvent: (_kind, data) => {
            if (data?.kind === "bus_batch" && Array.isArray(data.events)) {
              for (const ev of data.events) {
                const line = ev?.JobLogLine;
                if (line) this.jobLogLines.push(this.cleanLogEntry(line));
              }
              const MAX = 2000;
              if (this.jobLogLines.length > MAX) {
                this.jobLogLines.splice(0, this.jobLogLines.length - MAX);
              }
            }
          },
        }
      );
    },
    stopJobLogs() {
      this.jobLogStream?.close();
      this.jobLogStream = null;
    },

    // ── Settings ──────────────────────────────────────────────────────────
    // Porta de handlers/settings.luau + a load_settings() de connection.luau.
    // Buscado uma vez na conexão (não a cada snapshot — senão apagaria o que
    // o usuário está digitando nos formulários).

    async loadSettings() {
      const r = await this.api.rpc("GetDaemonSettings");
      if (r.ok && r.value?.DaemonSettings) {
        const s = r.value.DaemonSettings;
        this.ssPublicBase = s.public_base_url || "";
        this.ssEmail = s.acme_email || "";
        this.ssRegistryDomain = s.registry_domain || "";
      }
      await this.gpRefresh();
    },

    async settingsSave() {
      const email = this.ssEmail.trim();
      const registryDomain = this.ssRegistryDomain.trim();
      this.settingsMsg = "salvando…";
      const r = await this.api.rpcChecked({
        SetDaemonSettings: {
          acme_email: email || null,
          registry_domain: registryDomain || null,
        },
      });
      this.settingsMsg = r.ok ? "configurações salvas" : "erro: " + r.error;
    },

    async gpRefresh() {
      const r = await this.api.rpc("GitProviderList");
      this.gitProviders = (r.ok && r.value?.GitProviders) || [];
    },

    /** Mesmas validações de handlers/settings.luau::gp_connect: GitHub cai
     * pro github.com se a Base URL vier vazia (só existe pra Enterprise);
     * Gitea sempre exige Base URL. Client id+secret OU PAT, conforme gpMode. */
    async gpConnect() {
      this.gpOauthUrl = "";
      const isGithub = this.gpKind === "github";
      const kindWire = isGithub ? "Github" : "Gitea";
      const label = isGithub ? "GitHub" : "Gitea";
      let base = this.gpBaseUrl.trim();
      if (!base) {
        if (isGithub) base = "https://github.com";
        else {
          this.gpMsg = "informe a Base URL do Gitea";
          return;
        }
      }
      const name = this.gpName.trim() || label;
      const isOauth = this.gpMode !== "pat";
      let cmd;
      if (isOauth) {
        const cid = this.gpClientId.trim();
        const csec = this.gpClientSecret || "";
        if (!cid || !csec.trim()) {
          this.gpMsg = "Client ID e Client Secret são obrigatórios";
          return;
        }
        cmd = {
          GitProviderCreate: {
            kind: kindWire,
            name,
            base_url: base,
            auth_mode: "OAuth",
            oauth_client_id: cid,
            oauth_client_secret: csec,
            pat: null,
          },
        };
      } else {
        const pat = this.gpPat || "";
        if (!pat.trim()) {
          this.gpMsg = "informe o Personal Access Token";
          return;
        }
        cmd = {
          GitProviderCreate: {
            kind: kindWire,
            name,
            base_url: base,
            auth_mode: "Pat",
            oauth_client_id: null,
            oauth_client_secret: null,
            pat,
          },
        };
      }
      this.gpMsg = "conectando…";
      const r = await this.api.rpc(cmd);
      if (!r.ok || !r.value?.GitProviderInfo) {
        this.gpMsg = "erro: " + (r.ok ? "resposta inesperada" : r.error);
        return;
      }
      const pid = r.value.GitProviderInfo.id;
      if (isOauth) {
        // OAuth precisa de round-trip no navegador — diferente do Luau (que só
        // linkava por não saber abrir o browser), aqui abrimos direto.
        const ro = await this.api.rpc({ GitOAuthStart: { provider_id: pid } });
        if (ro.ok && ro.value?.OAuthUrl) {
          this.gpOauthUrl = ro.value.OAuthUrl;
          this.gpMsg = "autorize a janela aberta e depois clique em Atualizar";
          window.open(ro.value.OAuthUrl, "_blank");
        } else {
          this.gpMsg = "provider criado; inicie o OAuth manualmente";
        }
      } else {
        this.gpMsg = `conta ${label} conectada ✓`;
      }
      this.gpName = "";
      this.gpBaseUrl = "";
      this.gpClientId = "";
      this.gpClientSecret = "";
      this.gpPat = "";
      await this.gpRefresh();
    },

    async gpDelete(id) {
      const r = await this.api.rpcChecked({ GitProviderDelete: { id } });
      this.gpMsg = r.ok ? "provider removido" : "erro: " + r.error;
      await this.gpRefresh();
    },

    async iacExport() {
      this.iacExportMsg = "exportando…";
      const r = await this.api.rpcChecked("ManifestExportAll");
      if (!r.ok || !r.value?.ManifestBundle) {
        this.iacExportMsg = "erro: " + (r.ok ? "resposta inesperada" : r.error);
        return;
      }
      this.iacYaml = r.value.ManifestBundle.yaml;
      this.iacDotenv = r.value.ManifestBundle.dotenv;
      this.iacHasExport = true;
      this.iacExportMsg = "exportado ✓";
    },

    /** 3 formas de resposta possíveis (mesma distinção de handlers/
     * settings.luau::iac_import): MissingEnvVars (nada aplicado), Err
     * (rpc-level), ou ManifestReport (sucesso). */
    async iacImport() {
      this.iacHasMissing = false;
      this.iacHasReport = false;
      this.iacMissingVars = "";
      this.iacReportLines = [];

      const yaml = this.iacImportYaml || "";
      if (!yaml.trim()) {
        this.iacImportMsg = "cole o YAML do manifesto";
        return;
      }

      this.iacImportMsg = "importando…";
      const r = await this.api.rpc({
        ManifestImport: {
          yaml,
          dotenv: this.iacImportDotenv || "",
          prune: this.iacPrune,
          deploy: this.iacDeploy,
        },
      });
      if (!r.ok) {
        this.iacImportMsg = "erro: " + r.error;
        return;
      }
      const v = r.value;
      if (v?.MissingEnvVars) {
        this.iacHasMissing = true;
        this.iacMissingVars = v.MissingEnvVars.join(", ");
        this.iacImportMsg = "faltam variáveis — nada foi aplicado";
        return;
      }
      if (v?.Err) {
        this.iacImportMsg = `erro: ${v.Err.code}: ${v.Err.message}`;
        return;
      }
      if (!v?.ManifestReport) {
        this.iacImportMsg = "resposta inesperada do daemon";
        return;
      }
      const lines = (v.ManifestReport.actions || []).map(
        (a) => `[${a.action}] ${a.kind} ${a.name}`
      );
      if ((v.ManifestReport.deployed || []).length > 0) {
        lines.push("deploy disparado: " + v.ManifestReport.deployed.join(", "));
      }
      this.iacReportLines = lines;
      this.iacHasReport = true;
      this.iacImportMsg = "import concluído ✓";
      await this.refreshNow();
    },

    async serviceStop() {
      const id = this.selectedServiceId;
      const r = await this.api.rpcChecked({ ServiceStop: { service_id: id } });
      this.serviceMsg = r.ok ? "serviço parado" : "erro: " + r.error;
      await this.fetchServiceDetail(id);
      await this.refreshNow();
    },

    async serviceReload() {
      const id = this.selectedServiceId;
      const r = await this.api.rpcChecked({ ServiceReload: { service_id: id } });
      this.serviceMsg = r.ok ? "serviço recarregado" : "erro: " + r.error;
      await this.fetchServiceDetail(id);
    },

    // ── Logs (aba Logs do serviço) ───────────────────────────────────
    // Mesmo endpoint SSE dedicado do client iced (crates/daemon/src/api/
    // http_api.rs::service_logs), com o mesmo histórico inicial via LogsGet
    // (ver startServiceLogs).

    /** LogEntry/LogLine cru → { stream, line, timestamp } com ANSI limpo e
     * truncado (defesa contra uma linha absurda — ex. um dump binário sem
     * quebra — quebrar o layout inteiro da aba, como aconteceu antes de
     * filtrar ANSI: a barra de escape crua virava glifos de caixa e o texto
     * "empilhava" visualmente). Mesmo teto de fmt/service_detail.luau::
     * log_rows (LINE_MAX). */
    cleanLogEntry(e) {
      const LINE_MAX = 4000;
      let line = stripAnsi(e.line || "");
      if (line.length > LINE_MAX) line = line.slice(0, LINE_MAX) + "…";
      return { stream: e.stream, line, timestamp: e.timestamp };
    },

    /** Exposta no store (não só no módulo) porque o modal de logs de job é
     * global — ver bloco "Modais globais de Jobs" em index.html. */
    timeHms,

    async startServiceLogs() {
      this.stopServiceLogs();
      const id = this.selectedServiceId;
      // Semeia o histórico ANTES de abrir o stream ao vivo — mesma ordem do
      // client iced (handlers/services.luau::open_logs_window): sem isso, um
      // serviço já rodando há tempo (sem stdout novo desde então) mostra a
      // aba vazia para sempre, mesmo tendo logs de sobra.
      const seed = await this.api.rpc({ LogsGet: { service_id: id, tail: 500 } });
      this.serviceLogLines = ((seed.ok && seed.value?.Logs) || []).map(this.cleanLogEntry);
      // A troca de aba pode ter acontecido enquanto o fetch estava em voo —
      // se o usuário já saiu da aba Logs (ou do serviço), não abre a stream.
      if (this.serviceTab !== "logs" || this.selectedServiceId !== id) return;
      this.serviceLogStream = openStream(
        this.api.baseUrl,
        this.api.token,
        `/api/services/${id}/logs`,
        {
          onEvent: (_kind, data) => {
            if (data?.kind === "bus_batch" && Array.isArray(data.events)) {
              for (const ev of data.events) {
                const line = ev?.LogLine;
                if (line) this.serviceLogLines.push(this.cleanLogEntry(line));
              }
              const MAX = 2000;
              if (this.serviceLogLines.length > MAX) {
                this.serviceLogLines.splice(0, this.serviceLogLines.length - MAX);
              }
            }
          },
        }
      );
    },

    stopServiceLogs() {
      this.serviceLogStream?.close();
      this.serviceLogStream = null;
    },

    setServiceTab(tab) {
      this.serviceTab = tab;
      if (tab === "logs") this.startServiceLogs();
      else this.stopServiceLogs();
    },
  });
});

// Dispara `alpine:init` só agora — todos os listeners acima (login.js,
// dashboard.js, o Alpine.store deste arquivo) já estão registrados.
Alpine.start();

if ("serviceWorker" in navigator) {
  window.addEventListener("load", () => {
    navigator.serviceWorker.register("/sw.js").catch(() => {
      /* PWA opcional — a app funciona normalmente sem o SW */
    });
  });
}
