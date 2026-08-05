// screens/schedules.js — tela "Schedules": jobs one-shot (docker-compose)
// agendados ou manuais, de todos os projetos. Porta da seção
// `equals="schedules"` de home.gv. O wizard "novo job" e o modal de logs ao
// vivo moraram pro store (app.js) — a mesma aba "Jobs" do projeto
// (screens/project_detail.js) abre o mesmo wizard, então o estado não podia
// ficar preso a este componente. Este módulo só formata a tabela.
import { jobSummaryRows, timeHms } from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("schedules", () => ({
    get store() {
      return Alpine.store("app");
    },
    timeHms,

    get rows() {
      return jobSummaryRows(this.store.snap?.jobs, this.store.search);
    },
  }));
});
