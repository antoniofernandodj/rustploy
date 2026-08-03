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
import "./screens/projects.js";
import "./screens/project_detail.js";
import "./screens/new_service.js";
import "./screens/service_detail.js";
import Alpine from "https://cdn.jsdelivr.net/npm/alpinejs@3.x.x/dist/module.esm.js";
import { Api } from "./net/api.js";
import { openStream } from "./net/sse.js";
import { fmtUptime } from "./fmt.js";

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
      // Eventos "bus" (métricas/deploy) chegam em fases futuras junto das
      // telas que os consomem (Monitoring, Deploy Engine).
    },

    applySnapshot(msg) {
      this.snap = msg;
      const st = msg.status;
      if (st) {
        this.daemonVersion = st.version || "";
        this.daemonUptime = fmtUptime(st.uptime_secs);
        this.servicesLabel = `${st.services_running || 0}/${st.services_total || 0}`;
      }
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

    // ── Services ─────────────────────────────────────────────────────

    openNewService() {
      this.nav("new_service");
    },

    /** `source` já é o `ServiceSource` externally-tagged (ver new_service.js). */
    async createService(name, projectId, source, port, domain) {
      const spec = {
        name: name.trim(),
        project_id: projectId,
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
    // http_api.rs::service_logs); sem tail histórico, só o que chegar a
    // partir da conexão (mesmo comportamento do client iced).

    startServiceLogs() {
      this.stopServiceLogs();
      this.serviceLogLines = [];
      const id = this.selectedServiceId;
      this.serviceLogStream = openStream(
        this.api.baseUrl,
        this.api.token,
        `/api/services/${id}/logs`,
        {
          onEvent: (_kind, data) => {
            if (data?.kind === "bus_batch" && Array.isArray(data.events)) {
              for (const ev of data.events) {
                const line = ev?.LogLine;
                if (line) this.serviceLogLines.push(line);
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
