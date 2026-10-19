# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

Ordenada por valor (o mais valioso primeiro, avaliação de 2026-10-19). Ao
acrescentar um item, encaixe-o na posição certa em vez de colar no fim.

- [x] **PRIORIDADE ZERO — concluída em 2026-10-19 (falta só conferir o `.exe` do Windows): reduzir o Rust da GUI ao mínimo.** Análise de 2026-10-19. O
  Rust do `rustploy-gui` já é só a casca (~560 linhas de código real fora testes; o Luau/`.gvb`/`.gss`
  somam ~16.300). Para chegar a um `main.rs` de ~5 linhas (`glacier_ui::run("views/")`), o que sobrou
  precisa migrar para o glacier-ui. Do mais ao menos viável:
  1. ~~`src/manifest_zip.rs`~~ — **feito em 2026-10-19**: virou `views/scripts/manifest_zip.luau` sobre
     `zip_write`/`zip_read` do glacier-ui 0.123; testes Luau em `tests/` rodam com `glacier test` (CLI 0.7.1).
  2. ~~`src/desktop.rs`~~ e 3. ~~`src/embedded.rs` + `src/assets.rs`~~ — **feitos em 2026-10-19** (glacier-ui 0.125.1):
     `GlacierDaemon::install_desktop`, `embed_dirs!` e `locate_assets_and_chdir`. `embedded.rs` ficou só
     com a lista de pastas (~14 linhas); o `main.rs` tem 22. Sobra o `build.rs` (item 4).
  4. ~~`build.rs`~~ — **feito em 2026-10-19**: a lógica foi para o crate novo `glacier-build 0.1`
     (`stage_images`, `desktop_on_cargo_install`, `windows_resource`) e o `build.rs` ficou com 18 linhas.
     Não validado: o cross-build do `.exe` do Windows (a lógica de versão foi copiada igual).
  Observação: migrar troca o Rust do Rustploy por Rust do glacier-ui, o código continua existindo.
  Cada mudança no glacier-ui segue a regra do crate (nunca `path`/`[patch]`: publicar, depois subir
  a versão no `rustploy-gui`; `git fetch` e comparar os dois sentidos antes do push). O
  `[patch.crates-io]` de `vendor/iced_tiny_skia` é outro assunto (item mais abaixo, iced 0.15).

- [x] **CI de testes e lint** — feito em 2026-10-19: `.github/workflows/ci.yml` (fmt, clippy `-D warnings` e
  test por submodule, em push na main e PR). Falta só ver rodar no GitHub após o primeiro push.

- [ ] **Dependências entre serviços + autostart no boot.** Plano de 2026-07-11 em
  `docs/plano-dependencias-e-autostart.md`, nunca executado (não há `depends_on` nem `autostart` no
  daemon). Depois de um reboot da VPS a ordem de subida (banco antes da app) depende do `restart:` do
  container. Os passos que citam o TUI estão obsoletos; a UI vai nos dois clientes (GUI e webui).

- [ ] **Persistir o log de runtime dos containers.** O swap de deploy destrói o container antigo e o
  log dele morre junto, justamente quando mais se precisa dele. Maior valor e maior esforço entre os
  buracos conhecidos; contexto em `docs/plano-erro-de-deploy-invisivel.md`.

- [ ] **Exportar e importar DADOS de um container ou volume, para subir em outro banco/servidor.**
  O export atual (`rustploy.yml` + `rustploy.vars.toml`) leva só a configuração: volumes e bancos
  ficam de fora, então não dá para reproduzir a infra de uma VPS em outra máquina (nem testar uma
  migração com dados reais a partir do zip). Achar uma forma de baixar os dados e subi-los em outro
  lugar. Pontos a decidir:
  - **Volumes (qualquer coisa):** exportar um volume nomeado como `.tar.gz` (container descartável
    montando o volume: `tar czf - -C /data .`) e importar noutro volume/servidor; pela API do
    rustploy (stream) e pela GUI/webui ("Baixar volume" / "Enviar para volume"). Parar o serviço
    (ou usar snapshot) para o dado ficar consistente.
  - **Bancos (preferir dump lógico a copiar o diretório):** `pg_dump`/`pg_restore` (Postgres),
    `mysqldump`/`mysql` (MySQL/MariaDB), `mongodump`/`mongorestore`, `redis-cli --rdb`/RDB; escolhendo
    o engine pelo `db_kind` do serviço. Existe a migração entre servidores em `migration.rs`
    (`docker run ... pipe_script`) para bancos: reaproveitar, e estender para "exportar para arquivo"
    e "importar de arquivo", além de servidor -> servidor.
  - **Banco compartilhado** (`infra/pg-central`, `shared: true`): exportar/importar por banco/usuário.
  - Incluir no export da infra um modo "com dados" (opcional, pesado) ou um bundle à parte, com
    aviso de tamanho e de segredos; `secret:NOME` nunca é decifrado (regra do `ServiceBundle`).
  - Depois disso, o teste de migração Docker <-> Podman pode usar dados reais exportados da VPS.

- [ ] **Testar o backend Podman no que ficou de fora do smoke test** (o usuário vai
  fazer, 2026-10-18; detalhes em `docs/plano-backend-podman.md`, seção
  "Pendências"): upload de zip (Archive); TLS/ACME com domínio real; volumes e
  dados de bancos; jobs one-shot e registry embutido; subir os serviços reais (todos
  `Stopped`); `install.sh --engine docker` e `--engine podman` de ponta a ponta numa
  VPS limpa, depois da release.

- [ ] **Depois de migrar a VPS, conferir**: (1) `reboot` e ver se os containers com `restart:` voltam
  (`systemctl is-enabled podman-restart.service`); (2) de dentro de um container, `curl` para fora
  (saída pelo netavark/ufw); (3) portas publicadas pelo Compose (Supabase) e as `host_port` 20000-20004,
  de fora da VPS; (4) o que estava no `/etc/docker/daemon.json`.

- [ ] **Migração Docker -> Podman com dados reais** (`install.sh`/`make install-rustploy`): só
  foi testada com stubs. Rodar primeiro com `--dry-run`; o backup fica em `/var/backups/`.

- [ ] **Testar em máquinas de verdade o que só rodou em container.** (1) `install.ps1` num Windows real,
  principalmente o atalho do Menu Iniciar e o aviso do SmartScreen; (2) o `.deb` arm64 numa VPS arm64;
  (3) `install.sh` numa VPS com systemd (subir `rustployd` e o `rustployd-fw.socket` e confirmar com
  `systemctl status`). Detalhes em `docs/release.md`, seção "Ainda sem teste real".

- [ ] **`ServiceUpdate` é substituição total, não patch.** Um round-trip mal feito apaga configuração
  sem aviso. Oferecer atualização parcial ou, no mínimo, avisar/recusar quando campos existentes
  somem do payload.

- [ ] **Auditar `unwrap()` fora de teste no daemon**, começando por `features/iac/*`
  (`service_import.rs` 25, `manifest_import.rs` 25, `manifest.rs` 18) e `features/deploy/queue.rs`
  (18). São os maiores arquivos e lidam com entrada externa (manifestos importados): trocar por erro
  propagado. Avaliar também quebrar `manifest.rs` (1260 linhas) e `service_import.rs` (1013).

- [ ] **Registry embutido: integrar com o executor de deploy e push-to-deploy.** Fases 1 e 2 (auth e
  ingress), sub-aba da GUI e GC já estão prontos; falta isso. Ver `docs/plano-registry-embutido.md`.

- [ ] **Comando para remover/parar um container solto pela API.** `ServiceDelete` já para e remove os
  containers do serviço (corrigido); mas `RemoveContainer` (aba Containers) continua sem `force` e não há como
  parar um container que não pertence a um serviço.

- [ ] **A web UI segura o CSS/JS antigo depois que o daemon atualiza.** Visto em
  2026-10-16: o servidor já servia o `app.css` novo (54055 bytes, conferido por
  `curl`), mas o navegador continuou com o antigo mesmo depois de desregistrar o
  service worker e apagar os caches. Hipótese, não confirmada: `sw.js` faz
  `cache.addAll()` do app shell pelo cache HTTP, que é `immutable` por 1 ano, e as
  URLs não levam hash (`/app.css`, `/app.js`). Olhar `webui/sw.js`, o
  `Cache-Control` em `src/api/web_ui.rs`/`build.rs` e se faz sentido versionar as
  URLs dos assets (ou o SW buscar com `cache: 'reload'`). Hoje o remédio é
  Ctrl+Shift+R, às vezes mais de uma vez, e o texto de `interface/web-ui` já avisa.

- [ ] **Schema do protocolo.** A lista de comandos do manual do agente (`api/agent/manual.md`) é
  curada à mão e pode divergir dos `Command` reais. Gerar com `schemars` sobre a crate
  `rustploy-shared` (exige derivar nos tipos do protocolo e publicar o shared).

- [ ] **Migração Podman -> Docker.** Hoje só existe Docker -> Podman; voltar ao Docker é manual
  (o `install.sh` recusa: "voltar ao Docker não é automático"). Fazer o caminho inverso com as mesmas
  garantias: backup, `podman save | docker load` das imagens (antes de parar nada), cópia de TODOS os
  volumes (`cp -a`; opções NFS/bind recriadas sem copiar), redes de usuário/compose, parar o
  `rustployd` e os `rp_*`, rollback se falhar, trocar `[docker] socket_path` para
  `/var/run/docker.sock`, remover os drop-ins do Podman (`rustployd.service.d/podman.conf` e
  `podman.socket.d/rustploy.conf`), reiniciar e refazer o deploy dos serviços que estavam no ar. O
  esqueleto de `migrate_to_podman` em `install.sh` serve de base (espelhar `docker`/`podman`). Lembrar
  que o `podman-restart.service` e os arquivos de `/etc/containers/*.conf.d/rustploy.conf` não têm
  equivalente no Docker (o `dockerd` já religa e já tem docker.io como padrão), e que os
  nomes de imagem voltam a ser curtos.

- [ ] **Assinar o `.exe` do Windows** (SignPath, tier OSS): o job `sign-gui-windows` é pulado enquanto a variável de
  repositório `SIGNPATH_ORGANIZATION_ID` não existe. Passos em `docs/windows-code-signing.md`.

- [ ] **Capturar o diálogo "Remover job?" do app desktop e usar em
  `interface/schedules` (pt e en).** O desktop passou a pedir confirmação ao
  remover um job (`rustploy-gui`, `handlers/jobs.luau::job_delete`, commit
  `05bd708`), e a página já diz isso, mas sem imagem. Só dá para capturar depois
  de compilar e instalar o app novo. Borrar host, projeto e nome do job. Depois,
  refazer o deploy do site (`wrangler deploy` em `rustploy-docs/`). A confirmação
  em si nunca foi exercitada numa janela real (só os testes de renderização
  passaram): conferir Cancelar (painel ⋯ continua aberto) e Remover.

- [ ] **Resto do plano do site (`docs/plano-site-documentacao.md`).** Falta a
  captura de instalar a web UI como PWA (a barra do navegador não sai na
  extensão do Chrome: tem que ser tirada à mão), o CI de qualidade do site
  (links, exemplos executáveis, Lighthouse) e o domínio definitivo (canonical,
  sitemap e `Sitemap:` do `public/robots.txt`).

- [ ] **Remover o `vendor/iced_tiny_skia` quando o iced 0.15 sair.** O
  `text_editor` (`<textarea>`) vazava a linha parcial da borda no renderer de
  software; o upstream corrigiu na `master` (commit `23170119b`, 2026-01-28)
  mas nenhuma release 0.14.x tem. Conferir `cargo search iced` / crates.io.
  Quando houver release com o commit: (1) glacier-ui migra `iced` e publica;
  (2) rustploy sobe o `glacier-ui`; (3) no repo `rustploy-gui`, apagar `vendor/iced_tiny_skia/`,
  `vendor/README.md` e o `[patch.crates-io]` do `Cargo.toml`; (4) abrir a
  janela de logs e conferir que o recorte continua certo. Contexto em
  `vendor/README.md`.

- [ ] **Tirar o glacier-ui de dentro do iced (proposta, nada feito).** Dois caminhos, o A primeiro:
  (A) o glacier-ui passa a mandar no layout (Taffy) e nos eventos, e o iced vira só o backend de desenho e
  entrada; (B) depois, spikes de outros backends (Vello+Parley, Slint, egui) atrás do mesmo `trait Backend`.
  Fases, medições e decisões em aberto: `docs/plano-evolucao-do-backend-grafico.md`.

- [ ] **Página "Para quem vai contribuir" no site de docs.** Última seção do
  `docs/plano-site-documentacao.md` (§4, item 15) que não existe. Adiada de
  propósito: o projeto continua **fechado a contribuições** por enquanto (decisão
  de 2026-10-16). Quando retomar, a página só condensa a Parte 3 do `AGENTS.md`
  (arquitetura, build, testes, convenções) e aponta para ele, que continua sendo a
  fonte. Não copiar o texto inteiro, para não envelhecer em paralelo.

- [x] **Reorganizar o daemon e o `shared`** — feito em 2026-10-18 (`docs/plano-reorganizacao-daemon-e-shared.md`,
  seção "Como ficou"). O passo seguinte (contextos por feature) e a `web_ui` também já foram feitos.
