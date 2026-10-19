# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

- [ ] **Remover o `vendor/iced_tiny_skia` quando o iced 0.15 sair.** O
  `text_editor` (`<textarea>`) vazava a linha parcial da borda no renderer de
  software; o upstream corrigiu na `master` (commit `23170119b`, 2026-01-28)
  mas nenhuma release 0.14.x tem. Conferir `cargo search iced` / crates.io.
  Quando houver release com o commit: (1) glacier-ui migra `iced` e publica;
  (2) rustploy sobe o `glacier-ui`; (3) no repo `rustploy-gui`, apagar `vendor/iced_tiny_skia/`,
  `vendor/README.md` e o `[patch.crates-io]` do `Cargo.toml`; (4) abrir a
  janela de logs e conferir que o recorte continua certo. Contexto em
  `vendor/README.md`.

- [ ] **Capturar o diálogo "Remover job?" do app desktop e usar em
  `interface/schedules` (pt e en).** O desktop passou a pedir confirmação ao
  remover um job (`rustploy-gui`, `handlers/jobs.luau::job_delete`, commit
  `05bd708`), e a página já diz isso, mas sem imagem. Só dá para capturar depois
  de compilar e instalar o app novo. Borrar host, projeto e nome do job. Depois,
  refazer o deploy do site (`wrangler deploy` em `rustploy-docs/`). A confirmação
  em si nunca foi exercitada numa janela real (só os testes de renderização
  passaram): conferir Cancelar (painel ⋯ continua aberto) e Remover.

- [ ] **A web UI segura o CSS/JS antigo depois que o daemon atualiza.** Visto em
  2026-10-16: o servidor já servia o `app.css` novo (54055 bytes, conferido por
  `curl`), mas o navegador continuou com o antigo mesmo depois de desregistrar o
  service worker e apagar os caches. Hipótese, não confirmada: `sw.js` faz
  `cache.addAll()` do app shell pelo cache HTTP, que é `immutable` por 1 ano, e as
  URLs não levam hash (`/app.css`, `/app.js`). Olhar `webui/sw.js`, o
  `Cache-Control` em `src/api/web_ui.rs`/`build.rs` e se faz sentido versionar as
  URLs dos assets (ou o SW buscar com `cache: 'reload'`). Hoje o remédio é
  Ctrl+Shift+R, às vezes mais de uma vez, e o texto de `interface/web-ui` já avisa.

- [ ] **Página "Para quem vai contribuir" no site de docs.** Última seção do
  `docs/plano-site-documentacao.md` (§4, item 15) que não existe. Adiada de
  propósito: o projeto continua **fechado a contribuições** por enquanto (decisão
  de 2026-10-16). Quando retomar, a página só condensa a Parte 3 do `AGENTS.md`
  (arquitetura, build, testes, convenções) e aponta para ele, que continua sendo a
  fonte. Não copiar o texto inteiro, para não envelhecer em paralelo.

- [ ] **Resto do plano do site (`docs/plano-site-documentacao.md`).** Falta a
  captura de instalar a web UI como PWA (a barra do navegador não sai na
  extensão do Chrome: tem que ser tirada à mão), o CI de qualidade do site
  (links, exemplos executáveis, Lighthouse) e o domínio definitivo (canonical,
  sitemap e `Sitemap:` do `public/robots.txt`).

- [ ] **Testar em máquinas de verdade o que só rodou em container.** (1) `install.ps1` num Windows real,
  principalmente o atalho do Menu Iniciar e o aviso do SmartScreen; (2) o `.deb` arm64 numa VPS arm64;
  (3) `install.sh` numa VPS com systemd (subir `rustployd` e o `rustployd-fw.socket` e confirmar com
  `systemctl status`). Detalhes em `docs/release.md`, seção "Ainda sem teste real".

- [ ] **Assinar o `.exe` do Windows** (SignPath, tier OSS): o job `sign-gui-windows` é pulado enquanto a variável de
  repositório `SIGNPATH_ORGANIZATION_ID` não existe. Passos em `docs/windows-code-signing.md`.

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
- [ ] **Comando para remover/parar um container solto pela API.** `ServiceDelete` já para e remove os
  containers do serviço (corrigido); mas `RemoveContainer` (aba Containers) continua sem `force` e não há como
  parar um container que não pertence a um serviço.

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

- [ ] **Tirar o glacier-ui de dentro do iced (proposta, nada feito).** Dois caminhos, o A primeiro:
  (A) o glacier-ui passa a mandar no layout (Taffy) e nos eventos, e o iced vira só o backend de desenho e
  entrada; (B) depois, spikes de outros backends (Vello+Parley, Slint, egui) atrás do mesmo `trait Backend`.
  Fases, medições e decisões em aberto: `docs/plano-evolucao-do-backend-grafico.md`.

- [x] **Reorganizar o daemon e o `shared`** — feito em 2026-10-18 (`docs/plano-reorganizacao-daemon-e-shared.md`,
  seção "Como ficou"). O passo seguinte (contextos por feature) e a `web_ui` também já foram feitos.
