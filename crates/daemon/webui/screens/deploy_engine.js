// screens/deploy_engine.js — tela "Deploy Engine": fila global (um deploy por
// vez), execução em andamento e histórico das últimas 24h. Porta da seção
// `equals="deploy_engine"` de home.gv — tudo vem de `snap.engine`
// (DeployEngineSummary), já anexado ao Snapshot pelo daemon (sem RPC extra).
import { fmtUptime, engActiveRows, engQueuedRows, engRecentRows } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("deployEngine", () => ({
    get store() {
      return Alpine.store("app");
    },

    get engine() {
      return this.store.snap?.engine || null;
    },

    get active() {
      return engActiveRows(this.engine?.active);
    },
    get queued() {
      return engQueuedRows(this.engine?.queued);
    },
    get recent() {
      return engRecentRows(this.engine?.recent);
    },
    get paused() {
      return !!this.engine?.paused;
    },
    get uptime() {
      return this.engine ? fmtUptime(this.engine.uptime_secs) : "…";
    },
    get successCount() {
      return this.engine?.successful_24h ?? 0;
    },
    get failedCount() {
      return this.engine?.failed_24h ?? 0;
    },
    get totalCount() {
      return this.engine?.total_24h ?? 0;
    },
  }));
});
