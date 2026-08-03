// screens/schedules.js — tela "Schedules": jobs one-shot (docker-compose)
// agendados ou manuais, de todos os projetos. Porta da seção
// `equals="schedules"` de home.gv. Mutações vivem no store (app.js); este
// módulo formata pra exibição e guarda o estado do wizard "novo job"
// (equivalente ao `new_job_window` do cliente iced — aqui vira modal inline
// de 3 passos, sem motor de janela separado) e do modal de logs ao vivo.
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

    // ── Wizard "novo job" (3 passos: projeto → serviço → form) ──────────
    showNewJob: false,
    njobStep: "pick_project", // "pick_project" | "pick_service" | "form"
    njobProjectId: "",
    njobProjectName: "",
    njobServiceId: "",
    njobServiceName: "",
    njobName: "",
    njobCompose: "",
    njobMainService: "",
    njobKind: "manual", // "manual" | "interval" | "daily" | "weekly"
    njobHours: "6",
    njobHour: "3",
    njobMinute: "0",
    njobWeekday: "0",
    njobErr: "",
    submitting: false,

    get projects() {
      return this.store.snap?.projects || [];
    },
    /** Serviços do projeto escolhido — já em memória (snap.services), sem
     * re-fetch (diferente do Luau, que precisa pré-semear a janela isolada). */
    get servicesFiltered() {
      return (this.store.snap?.services || []).filter(
        (e) => e.service.spec.project_id === this.njobProjectId
      );
    },

    openNewJob() {
      this.showNewJob = true;
      this.njobStep = "pick_project";
      this.njobProjectId = "";
      this.njobProjectName = "";
      this.njobServiceId = "";
      this.njobServiceName = "";
      this.njobName = "";
      this.njobCompose = "";
      this.njobMainService = "";
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
      if (this.njobStep === "form") this.njobStep = "pick_service";
      else if (this.njobStep === "pick_service") this.njobStep = "pick_project";
    },

    /** Monta `recurrence` (Option<Recurrence>, externally-tagged) a partir
     * de njobKind — mesma lógica de njob_create do Luau. */
    buildRecurrence() {
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
      if (!this.njobName.trim() || !this.njobCompose.trim() || !this.njobMainService.trim()) {
        this.njobErr = "nome, compose e main service são obrigatórios";
        return;
      }
      this.njobErr = "";
      this.submitting = true;
      const r = await this.store.jobCreate({
        project_id: this.njobProjectId,
        // "" (não null) = job autônomo — Command::JobCreate::trigger_service_id
        // é String simples no protocolo (não Option<String>); o handler no
        // daemon é quem converte "" → None (job_create.rs).
        trigger_service_id: this.njobServiceId || "",
        name: this.njobName.trim(),
        compose: this.njobCompose,
        main_service: this.njobMainService.trim(),
        recurrence: this.buildRecurrence(),
      });
      this.submitting = false;
      if (r.ok) this.closeNewJob();
      else this.njobErr = r.error;
    },

    // ── Modal de logs ao vivo ────────────────────────────────────────────
    jobLogsFor: null,
    openJobLogs(jobRunId) {
      this.jobLogsFor = jobRunId;
      this.store.startJobLogs(jobRunId);
    },
    closeJobLogs() {
      this.jobLogsFor = null;
      this.store.stopJobLogs();
    },
  }));
});
