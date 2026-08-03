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
      this.stream?.close();
      this.stream = null;
      this.api = null;
      this.snap = null;
      this.connected = false;
      this.screen = "login";
      this.statusLine = "desconectado";
    },

    openStream() {
      this.stream?.close();
      this.stream = openStream(this.api.baseUrl, this.api.token, {
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
