// screens/service_detail.js — detalhe de um serviço. Porta parcial de
// service.gv (client iced): das 8 abas originais (General/Connection/
// Environment/Domains/Deployments/Healthcheck/Logs/Advanced), esta fase cobre
// General/Environment/Domains/Deployments/Logs — Connection/Healthcheck/
// Advanced e a edição de provider Git/Compose ficam para fases seguintes
// (ver plano). Estado de navegação/fetch/mutação mora no Alpine.store('app')
// (app.js); este módulo só formata para exibição e cuida dos formulários
// locais (novo env var, novo domínio).
import {
  serviceStatusLabelKind,
  sourceSummary,
  dateDmHms,
  fmtDuration,
  stateLabelKind,
  dotenvFromVars,
  parseDotenv,
  stripAnsi,
  timeHms,
  safeName,
  internalUrl,
  externalUrl,
  envRowsWithComments,
} from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("serviceDetail", () => ({
    get store() {
      return Alpine.store("app");
    },
    newEnvKey: "",
    newEnvValue: "",
    newDomain: "",
    newDomainPort: "",
    newDomainTls: false,
    buildLogText: "",
    buildLogFor: null,
    timeHms,

    get svc() {
      return this.store.serviceDetail;
    },
    get statusLabel() {
      return this.svc ? serviceStatusLabelKind(this.svc.status)[0] : "";
    },
    get statusKind() {
      return this.svc ? serviceStatusLabelKind(this.svc.status)[1] : "muted";
    },
    get sourceText() {
      return this.svc ? sourceSummary(this.svc.spec.source) : "—";
    },

    // ── Connection ────────────────────────────────────────────────────
    copyMsg: "",
    async copyToClipboard(text) {
      try {
        await navigator.clipboard.writeText(text || "");
        this.copyMsg = "copiado!";
        setTimeout(() => (this.copyMsg = ""), 1500);
      } catch {
        /* clipboard indisponível (ex. contexto não-seguro) — sem drama */
      }
    },

    /** Containers reais em execução para este serviço: só vêm no snapshot
     * (ManagedContainer[], anexado por http_api.rs::snapshot), não no
     * `Service` cru de `ServiceGet` — daí o cross-reference pelo id. */
    get liveContainers() {
      const s = this.store;
      const entry = ((s.snap && s.snap.services) || []).find((e) => e.service.id === this.svc?.id);
      const list = entry?.service.containers || [];
      const colorFor = (state) => {
        if (state === "running") return "ok";
        if (state === "restarting" || state === "created" || state === "paused") return "warn";
        return "muted";
      };
      return list.map((c) => ({
        id: c.id,
        idShort: (c.id || "").slice(0, 12),
        name: c.name || "—",
        state: c.state || "—",
        kind: colorFor(c.state),
      }));
    },

    get connectionInfo() {
      const svc = this.svc;
      if (!svc) return null;
      const spec = svc.spec;
      const domain = spec.domains?.[0]?.domain || spec.domain || "";
      const tls = spec.domains?.[0]?.tls ?? spec.tls_enabled ?? false;
      const safe = safeName(spec.name);
      return {
        port: spec.port,
        hostPort: spec.host_port || "—",
        domain: domain || "—",
        tls: tls ? "enabled" : "disabled",
        dbKind: spec.db_kind || null,
        internalUrl: internalUrl(spec.db_kind, safe, spec.port),
        externalUrl: externalUrl(domain, tls, spec.host_port, spec.db_kind, this.store.api.baseUrl, spec.env_vars),
      };
    },

    get envVars() {
      const svc = this.svc;
      if (!svc) return [];
      return envRowsWithComments(svc.spec.env_vars, svc.spec.env_comments);
    },

    async addEnvVar() {
      if (!this.newEnvKey.trim()) return;
      // JSON round-trip (não structuredClone): this.svc.spec é um Proxy
      // reativo do Alpine — o clonador estrutural nativo do browser não
      // sabe copiá-lo (DataCloneError). O JSON round-trip descarta a
      // reatividade de forma segura, já que o ServiceSpec é sempre dados
      // planos (sem funções/Date/referências circulares).
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.env_vars = spec.env_vars.filter((e) => e.key !== this.newEnvKey.trim());
      spec.env_vars.push({ key: this.newEnvKey.trim(), value: { Plain: this.newEnvValue } });
      const r = await this.store.saveServiceSpec(spec, "variável salva");
      if (r.ok) {
        this.newEnvKey = "";
        this.newEnvValue = "";
      }
    },

    async delEnvVar(key) {
      // JSON round-trip (não structuredClone): this.svc.spec é um Proxy
      // reativo do Alpine — o clonador estrutural nativo do browser não
      // sabe copiá-lo (DataCloneError). O JSON round-trip descarta a
      // reatividade de forma segura, já que o ServiceSpec é sempre dados
      // planos (sem funções/Date/referências circulares).
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.env_vars = spec.env_vars.filter((e) => e.key !== key);
      await this.store.saveServiceSpec(spec, "variável removida");
    },

    // ── Editor .env de texto (toggle "Exportar"/".env") ───────────────
    envTextOpen: false,
    envText: "",
    openEnvText() {
      this.envText = dotenvFromVars(this.svc.spec.env_vars, this.svc.spec.env_comments);
      this.envTextOpen = true;
    },
    closeEnvText() {
      this.envTextOpen = false;
    },
    async saveEnvText() {
      const { vars, comments } = parseDotenv(this.envText);
      // JSON round-trip (não structuredClone): this.svc.spec é um Proxy
      // reativo do Alpine — o clonador estrutural nativo do browser não
      // sabe copiá-lo (DataCloneError). O JSON round-trip descarta a
      // reatividade de forma segura, já que o ServiceSpec é sempre dados
      // planos (sem funções/Date/referências circulares).
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.env_vars = vars;
      spec.env_comments = comments;
      const r = await this.store.saveServiceSpec(spec, "variáveis salvas");
      if (r.ok) this.envTextOpen = false;
    },

    get domains() {
      const svc = this.svc;
      if (!svc) return [];
      return (svc.spec.domains || []).map((d) => ({
        domain: d.domain,
        port: d.port ?? svc.spec.port,
        tls: !!d.tls,
      }));
    },

    async addDomain() {
      if (!this.newDomain.trim()) return;
      // JSON round-trip (não structuredClone): this.svc.spec é um Proxy
      // reativo do Alpine — o clonador estrutural nativo do browser não
      // sabe copiá-lo (DataCloneError). O JSON round-trip descarta a
      // reatividade de forma segura, já que o ServiceSpec é sempre dados
      // planos (sem funções/Date/referências circulares).
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.domains = spec.domains || [];
      spec.domains.push({
        domain: this.newDomain.trim(),
        port: this.newDomainPort ? Number(this.newDomainPort) : null,
        tls: this.newDomainTls,
      });
      const r = await this.store.saveServiceSpec(spec, "domínio adicionado");
      if (r.ok) {
        this.newDomain = "";
        this.newDomainPort = "";
        this.newDomainTls = false;
      }
    },

    async delDomain(domain) {
      // JSON round-trip (não structuredClone): this.svc.spec é um Proxy
      // reativo do Alpine — o clonador estrutural nativo do browser não
      // sabe copiá-lo (DataCloneError). O JSON round-trip descarta a
      // reatividade de forma segura, já que o ServiceSpec é sempre dados
      // planos (sem funções/Date/referências circulares).
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.domains = (spec.domains || []).filter((d) => d.domain !== domain);
      await this.store.saveServiceSpec(spec, "domínio removido");
    },

    // ── Healthcheck ───────────────────────────────────────────────────
    hcKind: "none", // "none" | "tcp" | "http" | "docker"
    hcPath: "",
    hcStatus: "200",
    hcInterval: "5",
    hcTimeout: "3",
    hcRetries: "10",
    hcStart: "5",

    /** Popula o form local a partir do spec atual — chamado ao abrir a aba
     * (o form é editável, não reativo direto ao spec, então precisa de um
     * ponto explícito de sincronização). */
    initHcForm() {
      const hc = this.svc?.spec?.healthcheck;
      if (!hc) return;
      if (typeof hc.kind === "string") {
        this.hcKind = hc.kind === "DockerNative" ? "docker" : hc.kind.toLowerCase();
      } else if (hc.kind?.Http) {
        this.hcKind = "http";
        this.hcPath = hc.kind.Http.path;
        this.hcStatus = String(hc.kind.Http.expected_status);
      }
      this.hcInterval = String(hc.interval_secs);
      this.hcTimeout = String(hc.timeout_secs);
      this.hcRetries = String(hc.retries);
      this.hcStart = String(hc.start_period_secs);
    },

    async saveHealthcheck() {
      let kind;
      if (this.hcKind === "http") {
        kind = { Http: { path: this.hcPath.trim() || "/", expected_status: Number(this.hcStatus) || 200 } };
      } else if (this.hcKind === "docker") {
        kind = "DockerNative";
      } else if (this.hcKind === "tcp") {
        kind = "Tcp";
      } else {
        kind = "None";
      }
      const cur = this.svc.spec.healthcheck;
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.healthcheck = {
        kind,
        interval_secs: Number(this.hcInterval) || cur.interval_secs,
        timeout_secs: Number(this.hcTimeout) || cur.timeout_secs,
        retries: Number(this.hcRetries) || cur.retries,
        start_period_secs: Number(this.hcStart) || cur.start_period_secs,
      };
      await this.store.saveServiceSpec(spec, "healthcheck salvo");
    },

    // ── Advanced ──────────────────────────────────────────────────────
    advReplicas: "1",
    advRunCommand: "",

    initAdvForm() {
      const spec = this.svc?.spec;
      if (!spec) return;
      this.advReplicas = String(spec.replicas || 1);
      this.advRunCommand = spec.run_command || "";
      this.advPdcAddJobId = "";
    },

    get runArgsText() {
      const args = this.svc?.spec?.run_args || [];
      return args.length ? args.join(" ") : "(nenhum)";
    },

    async saveAdvanced() {
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      let r = Number(this.advReplicas) || 1;
      if (r < 1) r = 1;
      spec.replicas = Math.floor(r);
      const rc = this.advRunCommand.trim();
      spec.run_command = rc || null;
      await this.store.saveServiceSpec(spec, "advanced salvo");
    },

    // ── Fila de pré-deploy check ────────────────────────────────────────
    // Efeito imediato (como Domains), não passa pelo Save acima — cada
    // add/remove/reorder já salva na hora. Ver docs/plano-pre-deploy-gate.md.

    // Fila efetiva de ids: `pre_deploy_job_ids` quando não vazia, senão cai
    // no `pre_deploy_job_id` legado (retrocompat) — mesmo idioma de
    // `ServiceSpec::pre_deploy_checks` no lado Rust.
    get preDeployQueueIds() {
      const spec = this.svc?.spec;
      if (!spec) return [];
      if (spec.pre_deploy_job_ids && spec.pre_deploy_job_ids.length > 0) {
        return spec.pre_deploy_job_ids;
      }
      if (spec.pre_deploy_job_id) return [spec.pre_deploy_job_id];
      return [];
    },

    // Fila com nome resolvido a partir dos Jobs do projeto (vem do snapshot
    // já em memória, mesmo padrão de `servicesFiltered` em schedules.js). Um
    // job apagado por fora mas ainda referenciado aparece com rótulo
    // "(job removido)" em vez de sumir, pra dar pra removê-lo da fila.
    get preDeployChecks() {
      const byId = new Map(
        (this.store.snap?.jobs || []).map((s) => [s.job.id, s.job.name])
      );
      return this.preDeployQueueIds.map((id, i) => ({
        id,
        index: i + 1,
        name: byId.get(id) || `(job removido: ${id})`,
      }));
    },

    // Jobs do projeto que AINDA NÃO estão na fila — opções do seletor
    // "adicionar à fila" (evita duplicar o mesmo job na sequência).
    get preDeployAvailableJobs() {
      const projectId = this.svc?.spec?.project_id;
      if (!projectId) return [];
      const inQueue = new Set(this.preDeployQueueIds);
      return (this.store.snap?.jobs || [])
        .filter((s) => s.job.project_id === projectId && !inQueue.has(s.job.id))
        .map((s) => ({ id: s.job.id, name: s.job.name }));
    },

    advPdcAddJobId: "",

    async pdcAdd() {
      const jobId = this.advPdcAddJobId;
      if (!jobId) return;
      const ids = this.preDeployQueueIds.slice();
      if (!ids.includes(jobId)) ids.push(jobId);
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.pre_deploy_job_ids = ids;
      spec.pre_deploy_job_id = null;
      const r = await this.store.saveServiceSpec(spec, "check adicionado à fila");
      if (r.ok) this.advPdcAddJobId = "";
    },

    async pdcDel(jobId) {
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.pre_deploy_job_ids = this.preDeployQueueIds.filter((id) => id !== jobId);
      spec.pre_deploy_job_id = null;
      await this.store.saveServiceSpec(spec, "check removido da fila");
    },

    // Sem drag-and-drop na web UI (sem lib de DnD): reordena com botões
    // mover-pra-cima/baixo — mesmo resultado final do arraste na GUI iced.
    async pdcMove(jobId, delta) {
      const ids = this.preDeployQueueIds.slice();
      const i = ids.indexOf(jobId);
      const j = i + delta;
      if (i < 0 || j < 0 || j >= ids.length) return;
      [ids[i], ids[j]] = [ids[j], ids[i]];
      const spec = JSON.parse(JSON.stringify(this.svc.spec));
      spec.pre_deploy_job_ids = ids;
      spec.pre_deploy_job_id = null;
      await this.store.saveServiceSpec(spec, "fila reordenada");
    },

    get deployments() {
      return (this.store.serviceDeployments || []).map((d) => {
        const [label, kind] = stateLabelKind(d.state);
        return {
          id: d.id,
          image: d.image,
          stateLabel: label,
          stateKind: kind,
          duration: fmtDuration(d),
          start: dateDmHms(d.started_at),
          terminal: d.state === "Live" || d.state === "Failed" || d.state === "Stopped",
        };
      });
    },

    async viewBuildLog(deploymentId) {
      const r = await this.store.api.rpc({ GetBuildLogs: { deployment_id: deploymentId } });
      if (r.ok && r.value?.BuildLogs) {
        // `docker build` também manda cores ANSI — mesmo tratamento dos logs
        // de runtime (ver app.js::cleanLogEntry).
        this.buildLogText = r.value.BuildLogs.map((l) => stripAnsi(l.line)).join("\n");
        this.buildLogFor = deploymentId;
      }
    },
    closeBuildLog() {
      this.buildLogFor = null;
      this.buildLogText = "";
    },

    async abortDeployment(deploymentId) {
      await this.store.deployAbort(deploymentId);
    },
    async removeDeployment(deploymentId) {
      await this.store.deleteDeployment(deploymentId);
    },
  }));
});
