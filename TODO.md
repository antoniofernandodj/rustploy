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
