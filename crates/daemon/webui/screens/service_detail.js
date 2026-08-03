// screens/service_detail.js — detalhe de um serviço. Porta parcial de
// service.gv (client iced): das 8 abas originais (General/Connection/
// Environment/Domains/Deployments/Healthcheck/Logs/Advanced), esta fase cobre
// General/Environment/Domains/Deployments/Logs — Connection/Healthcheck/
// Advanced e a edição de provider Git/Compose ficam para fases seguintes
// (ver plano). Estado de navegação/fetch/mutação mora no Alpine.store('app')
// (app.js); este módulo só formata para exibição e cuida dos formulários
// locais (novo env var, novo domínio).
import { serviceStatusLabelKind, sourceSummary, dateDmHms, fmtDuration, stateLabelKind } from "../fmt.js";

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

    get envVars() {
      const svc = this.svc;
      if (!svc) return [];
      return (svc.spec.env_vars || []).map((e) => ({
        key: e.key,
        value: e.value?.Plain ?? (e.value?.Secret ? `<secret:${e.value.Secret}>` : ""),
        isSecret: !!e.value?.Secret,
      }));
    },

    async addEnvVar() {
      if (!this.newEnvKey.trim()) return;
      const spec = structuredClone(this.svc.spec);
      spec.env_vars = spec.env_vars.filter((e) => e.key !== this.newEnvKey.trim());
      spec.env_vars.push({ key: this.newEnvKey.trim(), value: { Plain: this.newEnvValue } });
      const r = await this.store.saveServiceSpec(spec, "variável salva");
      if (r.ok) {
        this.newEnvKey = "";
        this.newEnvValue = "";
      }
    },

    async delEnvVar(key) {
      const spec = structuredClone(this.svc.spec);
      spec.env_vars = spec.env_vars.filter((e) => e.key !== key);
      await this.store.saveServiceSpec(spec, "variável removida");
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
      const spec = structuredClone(this.svc.spec);
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
      const spec = structuredClone(this.svc.spec);
      spec.domains = (spec.domains || []).filter((d) => d.domain !== domain);
      await this.store.saveServiceSpec(spec, "domínio removido");
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
        this.buildLogText = r.value.BuildLogs.map((l) => l.line).join("\n");
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
