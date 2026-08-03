// screens/monitoring.js — tela "Monitoring": uso de CPU/memória do host e
// por container. Porta da seção `equals="monitoring"` de home.gv. As
// métricas por serviço só existem depois do primeiro evento `ContainerMetrics`
// (ver app.js::applyBusEvent) — o snapshot periódico não as carrega.
import { pairList, monitoringRows } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("monitoring", () => ({
    get store() {
      return Alpine.store("app");
    },

    get rows() {
      const pairs = pairList(this.store.snap?.services);
      return monitoringRows(pairs, this.store.metricsById);
    },
  }));
});
