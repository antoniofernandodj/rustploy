// screens/dashboard.js — tela "Deployments" (view padrão do shell). Porta de
// fmt.deployments (crates/rustploy-gui/views/scripts/fmt/dashboard.luau) +
// da seção `equals="deployments"` de shell.gv.
import { dateDmHms, fmtDuration, stateLabelKind } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("dashboard", () => ({
    get store() {
      return Alpine.store("app");
    },

    /** Linhas formatadas da tabela, filtradas pelo termo de busca da topbar. */
    get rows() {
      const s = this.store;
      const term = (s.search || "").toLowerCase();
      const list = (s.snap && s.snap.deployments) || [];
      return list
        .filter((entry) => {
          if (!term) return true;
          return (
            entry.service_name.toLowerCase().includes(term) ||
            entry.project_name.toLowerCase().includes(term)
          );
        })
        .map((entry) => {
          const d = entry.deployment;
          const [label, kind] = stateLabelKind(d.state);
          return {
            service: entry.service_name,
            project: entry.project_name,
            stateLabel: label,
            stateKind: kind,
            duration: fmtDuration(d),
            start: dateDmHms(d.started_at),
          };
        });
    },
  }));
});
