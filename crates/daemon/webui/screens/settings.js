// screens/settings.js — tela "Settings": Web Server / Git / Infra as Code.
// Porta da seção `equals="settings"` de home.gv. Todo o estado editável mora
// no store (app.js, x-model direto) — este módulo só formata os provedores
// Git pra exibição.
import { gitProviderRows } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("settings", () => ({
    get store() {
      return Alpine.store("app");
    },

    get gitProviderRows() {
      return gitProviderRows(this.store.gitProviders);
    },
  }));
});
