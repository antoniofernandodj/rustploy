// screens/ingress.js — tela "Ingress": rotas ativas no reverse proxy (por
// domínio) e portas TCP de host expostas diretamente. Porta da seção
// `equals="ingress"` de home.gv.
import { pairList, ingressRows, hostPortRows } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("ingress", () => ({
    get store() {
      return Alpine.store("app");
    },

    get pairs() {
      return pairList(this.store.snap?.services);
    },
    get routes() {
      return ingressRows(this.pairs);
    },
    get hostPorts() {
      return hostPortRows(this.pairs);
    },
  }));
});
