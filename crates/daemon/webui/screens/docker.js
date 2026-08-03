// screens/docker.js — tela "Docker": containers/imagens/volumes/networks do
// host inteiro (não só recursos geridos pelo rustploy) + sub-aba Registry.
// Porta da seção `equals="docker"` de home.gv. Mutações vivem no store
// (app.js); este módulo só formata pra exibição e guarda o estado local do
// modal "novo token de registry" (equivalente ao `new_registry_token_window`
// do cliente iced — aqui vira modal inline, sem motor de janela separado).
import {
  dockerContainerRows,
  dockerImageRows,
  dockerVolumeRows,
  dockerNetworkRows,
  registryRepoRows,
  registryTagRows,
  registryTokenRows,
} from "../fmt.js";

document.addEventListener("alpine:init", () => {
  Alpine.data("docker", () => ({
    get store() {
      return Alpine.store("app");
    },

    get containers() {
      return dockerContainerRows(this.store.snap?.docker_containers, this.store.search);
    },
    get images() {
      const rows = dockerImageRows(this.store.snap?.docker_images, this.store.search);
      return this.store.onlyUsedImages ? rows.filter((r) => r.inUse) : rows;
    },
    get volumes() {
      const rows = dockerVolumeRows(this.store.snap?.docker_volumes, this.store.search);
      return this.store.onlyUsedVolumes ? rows.filter((r) => r.inUse) : rows;
    },
    get networks() {
      const rows = dockerNetworkRows(this.store.snap?.docker_networks, this.store.search);
      return this.store.onlyUsedNetworks ? rows.filter((r) => r.inUse) : rows;
    },
    get repos() {
      return registryRepoRows(this.store.snap?.registry_repos, this.store.search);
    },
    get tags() {
      return registryTagRows(this.store.registryTags);
    },
    get tokens() {
      return registryTokenRows(this.store.registryTokens);
    },
    get registryHost() {
      const rs = this.store.snap?.registry_status;
      if (!rs) return "127.0.0.1:5100";
      return rs.domain && rs.domain.trim() ? rs.domain : `127.0.0.1:${rs.port}`;
    },
    get registryStatusLabel() {
      const rs = this.store.snap?.registry_status;
      if (!rs) return "desabilitado";
      return rs.enabled ? `ativo em ${this.registryHost}` : "desabilitado";
    },

    // ── Modal "novo token" ───────────────────────────────────────────────
    showTokenModal: false,
    ntokStep: "form", // "form" | "reveal"
    ntokName: "",
    ntokScope: "pull",
    ntokErr: "",
    ntokLoginCmd: "",

    openTokenModal() {
      this.showTokenModal = true;
      this.ntokStep = "form";
      this.ntokName = "";
      this.ntokScope = "pull";
      this.ntokErr = "";
      this.ntokLoginCmd = "";
    },
    closeTokenModal() {
      this.showTokenModal = false;
    },
    async ntokCreate() {
      if (!this.ntokName.trim()) {
        this.ntokErr = "nome obrigatório";
        return;
      }
      this.ntokErr = "";
      const r = await this.store.registryCreateToken(this.ntokName.trim(), this.ntokScope);
      if (!r.ok) {
        this.ntokErr = r.error;
        return;
      }
      this.ntokLoginCmd = `docker login ${this.registryHost} -u ${this.ntokName.trim()} -p ${r.secret}`;
      this.ntokStep = "reveal";
    },
  }));
});
