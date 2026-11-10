# Como publicar uma release

> Escrito em 2026-10-16, quando saiu a primeira (`v0.1.1`). O que está aqui foi
> aprendido rodando o pipeline de verdade; os achados abaixo custaram várias execuções.

Uma release é uma **tag `v*` no agregador**. O `.github/workflows/release.yml` compila e
publica sozinho em *GitHub Releases*:

| Arquivo | De onde vem |
|---|---|
| `rustployd_<versão>_amd64.deb` e `_arm64.deb` | `cargo deb` do daemon (arm64 compila nativo no runner `ubuntu-24.04-arm`) |
| `rustploy-gui_<versão>_amd64.deb` | `cargo deb` do app |
| `rustploy-gui-windows-<tag>.zip` | build cruzado com `cargo-xwin`, assinado só se o SignPath estiver configurado |
| `checksums.sha256` | `sha256sum` de todos acima |
| `<arquivo>.minisig` | assinatura de cada `.deb` e do zip, feita por `tools/sign-release.sh` (ver "Assinatura dos pacotes") |

Os instaladores (`install.sh`, `install.ps1`) leem a **última** release pela API do GitHub e
procuram os arquivos pelo padrão do nome, então nada precisa ser editado a cada release.

## Passo a passo

1. Tudo commitado e enviado (submódulos primeiro, depois o agregador com os ponteiros).
2. **Mexeu no updater** (`rustployd-update`, `update-core`, as units ou o `postinst`)? Rode antes o teste de ponta a ponta:
   `rustploy-daemon/crates/daemon/tests/e2e-updater/run.sh` (uns 4 minutos; precisa de podman, minisign e socat). Ele sobe um
   contêiner com systemd de verdade, instala pacotes de mentira assinados com uma chave descartável e confere 11 cenários: caminho
   feliz, assinatura de outra chave, outro produto, downgrade, caminho fora do diretório, `postinst` que falha, daemon que cai em
   laço (volta atrás sozinho), rollback manual e sem backup. O helper de teste só difere do real na chave embutida, definida em
   tempo de compilação por `RUSTPLOY_E2E_TRUSTED_KEY` (o CI e o `make deb` nunca a definem; um teste confere).
   Mexeu no **helper do app** (`rustploy-gui-update`, a política do polkit)? Rode também
   `rustploy-gui/tests/e2e-updater-gui/run.sh`: contêiner com systemd e `polkitd`, `pkexec` e a política reais, com uma regra de
   teste que autoriza o usuário `pessoa` sem senha. Confere 11 cenários, entre eles cache de outro usuário (o diretório vem do
   `PKEXEC_UID`, não do `HOME`), link simbólico, usuário sem autorização e `postinst` que falha. (Na imagem de teste o `polkitd`
   sobe à mão como root, porque o hardening da unit dele não roda num contêiner sem privilégios.)
3. **Teste sem publicar:** `gh workflow run release.yml --ref main`. O job `release` só roda em tag,
   então isso só compila e guarda os pacotes como artefatos (`gh run download <id>`).
4. Confira os artefatos (`dpkg-deb -I` mostra `Depends`, inclusive a versão mínima da `libc6`).
5. `git tag -a vX.Y.Z -m "..." && git push origin vX.Y.Z` faz o CI compilar e criar a release como
   **rascunho** (ela ainda não aparece em `releases/latest`).
6. **Assine e publique:** `tools/sign-release.sh vX.Y.Z`. O script baixa os pacotes do rascunho, confere o
   `checksums.sha256`, assina cada `.deb` e o zip do Windows com a sua chave (pede a senha a cada
   arquivo), confere cada assinatura com a chave pública, sobe os `.minisig` e publica. Use
   `--dry-run` para só assinar, ou `--no-publish` para subir as assinaturas e deixar o rascunho.
7. Teste o comando de uma linha de verdade num Debian limpo (`docker run debian:12`).

## Assinatura dos pacotes

Os updaters (`docs/plano-updaters-daemon-e-gui.md`) só instalam o que estiver **assinado** com a chave
do dono; o SHA-256 sozinho não basta, porque sai da mesma release do arquivo. Por isso:

- A chave é um par **minisign (ed25519)** gerado na máquina do dono, **nunca no CI**:
  `minisign -G -p ~/.minisign/rustploy.pub -s ~/.minisign/rustploy.key` (a privada pede senha).
  Guarde uma cópia offline de cada privada, em lugares diferentes. A **pública** é a única parte que vai para o código dos updaters e para as docs.
- Instale a ferramenta: `sudo apt install minisign`.
- O CI cria a release como rascunho justamente porque não tem a chave. Uma release publicada **sem**
  `.minisig` não é oferecida pelo updater (o `install.sh` manual continua funcionando).
- Cada `.minisig` carrega um comentário confiável `package=… version=… arch=…`, assinado junto; o updater
  compara esses campos com os do próprio pacote antes de instalar.
- **Duas chaves, uma por máquina** (desde 2026-10-27). Cada máquina de onde se lança release tem a sua, e
  os updaters, o `install.sh` e o `install.ps1` aceitam qualquer uma das duas:

  | Máquina | id | Pública | Privada (só na máquina) |
  |---|---|---|---|
  | 1 (antonio) | `C6A30203D40B0196` | `keys/rustploy-update.pub` | `~/.minisign/rustploy.key` |
  | 2 (itemize) | `8C7A43C31526B2B5` | `keys/rustploy-update-maquina-2.pub` | `~/.minisign/rustploy-reserva.key` |

  Na máquina 2, assine com `tools/sign-release.sh vX.Y.Z --key ~/.minisign/rustploy-reserva.key` (o padrão
  do script é `~/.minisign/rustploy.key`). As públicas embutidas ficam em `TRUSTED_PUBLIC_KEYS`
  (`rustploy-shared/update-core/src/signature.rs`), em `TRUSTED_KEYS` no `install.sh` e em `$TrustedKeys` no
  `install.ps1`; mudou uma, mude as quatro (com `keys/`).
- **Uma chave nova só vale para quem já instalou uma versão que a traz.** A chave da máquina 2 entrou na
  v0.1.21 (`rustploy-update-core` 0.3.3), e essa release já foi assinada pela **máquina 2**: nenhuma
  instalação rodava ainda uma versão com updater (o servidor estava no daemon 0.1.8), então ninguém
  dependia da chave da máquina 1. Uma instalação parada entre a v0.1.17 e a v0.1.20 só conhece a máquina 1
  e recusa a v0.1.21 pelo botão: atualize-a pelo `install.sh`, que lê as chaves da `main`.
- **Rotação** (uma chave perdida ou vazada): pela máquina que restou, publique uma versão que tire a chave
  ruim das quatro listas (e, se quiser, traga uma nova). Enquanto as instalações não atualizarem, a chave
  ruim continua aceita por elas: num vazamento, publique logo.
- Conferir à mão: `minisign -V -p keys/rustploy-update.pub -m rustployd_X.Y.Z_amd64.deb` (ou
  `keys/rustploy-update-maquina-2.pub`, conforme quem assinou; o erro "key id … is X" diz qual).
  Com `-P "$(sed -n 2p keys/…pub)"`, o `-P` quer o **texto** da chave, não o nome do arquivo.
- **A senha da chave não fica em arquivo dentro do repositório**: use o gerenciador de senhas.
- **Nunca edite uma release publicada**; se errou, crie outra versão (regra de sempre).

Uma tag cuja release **não** foi publicada pode ser apagada e refeita no commit corrigido
(`git tag -d`, `git push origin :refs/tags/vX.Y.Z`). Depois de publicada, não mexa nela: crie outra.

## Por que os .deb compilam num Ubuntu 22.04

A versão mínima da glibc de um binário vem de **onde ele é compilado**. Um build no runner novo
(24.04, glibc 2.39) exigiria `libc6 >= 2.39` e não instalaria no Debian 12. Dentro de um container
`ubuntu:22.04` o daemon exige `libc6 >= 2.34` e o app `>= 2.35`. Isso aparece na tabela de
compatibilidade em `rustploy-docs` (`instalacao/requisitos`). Mudou o container? Mudou a tabela.

## Armadilhas que já apareceram

- **O `update-core` também é compilado para o Windows** (o app depende dele). Antes de publicá-lo, rode
  `cargo check --target x86_64-pc-windows-gnu` em `rustploy-shared/update-core` (`rustup target add x86_64-pc-windows-gnu`): a 0.3.0
  saiu com um `std::os::unix` solto no módulo `outcome` e só o CI do Windows pegou, depois de publicada.
- **O CI do Windows compila todos os binários do pacote** (`cargo xwin build` sem `--bin`). O `rustploy-gui-update` só existe no
  Unix (`geteuid`, `getpwuid_r`), então o `main.rs` dele tem um *stub* para outros sistemas e o corpo mora em `helper.rs`, atrás de
  `#[cfg(unix)]`. Binário novo que use API de Unix precisa do mesmo cuidado.
- **Esperar um processo com `pgrep -f`** num laço de shell casa com a própria linha de comando do laço e nunca termina; espere pelo PID.
- **Comparar a tag com a versão do programa dá "há versão nova" para sempre.** O teste real v0.1.17 -> v0.1.18 mostrou: depois de
  atualizar para o daemon 0.1.12, o `CheckUpdate` ainda dizia `latest 0.1.18, update available`. A verificação compara a versão do
  **pacote** que a release traz (`update_core::release::package_version`, lida do nome `rustployd_<v>-1_<arq>.deb`), nunca a tag.
- **A tag não é a versão do pacote.** A tag do agregador (`v0.1.16`) e a versão dos crates (`0.1.10`, `0.1.20`) são
  numerações separadas, e o `cargo-deb` ainda acrescenta a revisão (`rustployd_0.1.10-1_amd64.deb`). Por isso o
  `tools/sign-release.sh` não compara a versão do `.deb` com a tag.

- **`ubuntu-latest` mudou de versão por baixo** (v0.1.32, 2026-11-02: virou 26.04 "resolute"). O `llvm.sh` do
  apt.llvm.org não tem repositório para ele (404) e o build do Windows caiu no `apt-get update`. O job
  `build-gui-windows` agora usa `ubuntu-24.04` fixo; ao subir de versão, valide o LLVM 20 antes.
- **O `install.ps1` num PowerShell de Linux** (`mcr.microsoft.com/powershell`) só prova a descoberta da release, o
  download e o SHA-256: o `minisign.exe` não executa lá. A conferência da assinatura precisa de um Windows de verdade.
- **`install.sh --engine podman` no Debian 12 é recusado** (podman 4.3 < 4.7 do `podman compose`), por desenho. Para
  testar o instalador no `debian:12`, use o comando padrão (Docker).

- **O job `release` era pulado.** O `sign-gui-windows` (opcional) é pulado sem SignPath, e o `if` padrão
  (`success()`) exige sucesso de **todos os ancestrais**. O job usa `always()` com a checagem explícita
  de cada `needs.<job>.result`.
- **O `.deb` do app precisa das libs de sistema** num container vazio: `libgtk-3-dev`,
  `libayatana-appindicator3-dev`, `libxdo-dev`, `libfontconfig1-dev`. O runner antigo já as trazia.
- **Build do Windows:** `ring` pede `llvm-lib` e `aws-lc-sys` pede `nasm`; os headers do MSVC STL do
  `cargo-xwin` exigem **Clang >= 19** (o do runner é o 18), então instala o LLVM 20 do apt.llvm.org e o põe no `PATH`.
- **A API do GitHub devolve o JSON numa linha só.** O `install.sh` usa `grep -o`; um `sed` por linha pega só o
  último arquivo. Os testes locais com JSON indentado passavam e o real falhava: teste contra a release de verdade.
- **O `/etc/os-release` define `VERSION`** e sobrescrevia a variável do script. Ele é lido numa subshell.
- **O pacote sobe sem `[api]`** (loopback e sem token), o que não deixa nenhum cliente entrar. O instalador gera o token
  e grava o `[api]`; `--remote` abre o bind. Numa atualização mantém o `config.toml` (`--force-confold`).

## v0.1.3 (2026-10-18): backend Podman

- **`make lint` já não passava na `main`** (fmt sujo nos três repos e ~60 erros de clippy 1.95). Foi
  corrigido de uma vez (fmt, `clippy --fix`, uns `#[allow]` em `from_str`/`Response`/funções com muitos
  argumentos): daí o `rustploy-shared` 0.4.1 e o `rustploy-gui` 0.1.7, sem mudança de comportamento.
- **Três testes de `web_ui::headless_tests` falham por ambiente** (também na `main` anterior); o `make test`
  para no primeiro repo que falha, então rode `cargo test` no `shared` e `cargo test --bins` + `--test
  app_manifest` no `gui` à parte.
- **O `cargo publish` do daemon demora** (recompila o pacote); se a chamada for interrompida, confira no
  crates.io (`cargo search rustploy`) antes de repetir.
- **`install.sh --deb ARQUIVO`** instala um `.deb` local (teste de um build antes de existir release) e
  **`install.sh engine`** só escolhe/troca o motor de um rustployd já instalado (é o que o `make
  install-rustploy` chama). O instalador é servido da `main`, então mudá-lo não exige nova release.
- O comando de uma linha foi testado pela URL pública em `debian:12` (daemon com Docker, e o app) e em
  `debian:13` (`--engine podman`); o `install.ps1` chegou até a extração num PowerShell no Linux.

## v0.1.7 (2026-10-20): aquecimento de serviços e tela Em execução

- **`cargo update -p rustploy-shared` é ambíguo** no workspace do daemon: o `importer` ainda usa o
  shared 0.5.0. Use `cargo update -p rustploy-shared@0.7.0 --precise 0.8.0` (a versão antiga que sai).
- **Sem o shared publicado o daemon não compila** (`make deb-rustploy` usa o crates.io): publique o
  shared antes de qualquer build do daemon; para testar antes, `--config
  'patch.crates-io.rustploy-shared.path="../rustploy-shared"'` (e desfaça o `Cargo.lock`).
- **Os `target/debug` do daemon e da GUI passam de 20 GB cada**; o `clippy` + `test` dos dois enchem o
  disco de 457 GB se houver pouca folga (erro `No space left on device` no meio do build).
- O comando de uma linha foi testado em `debian:12` (via podman, sem docker nesta máquina): instalou o
  daemon 0.1.6 e ele iniciou. O app e o `install.ps1` não foram retestados nesta release.

## v0.1.31 (2026-10-31): interface em inglês e pt-BR (i18n)

- **Ordem que funcionou:** `glacier-ui` 0.147.0 (publicado primeiro, a GUI o exige) → testes (`glacier test`, `cargo test`
  da GUI, `cargo test -p rustploy web_ui`) → `make lint` → commits (gui, daemon, docs) e push → ponteiros + `make index`
  no agregador → `cargo publish` da GUI (0.1.35) e do daemon (0.1.20) → `gh workflow run release.yml --ref main` (verde) → tag
  `v0.1.31` → rascunho → `tools/sign-release.sh` → publicada.
- **`make test` caiu no link da GUI** (`ld ... signal 7 [Bus error]`) quando eu rodava o build do site e outros testes ao mesmo
  tempo (disco a 92%). Não é do código: rode a GUI sozinha (`cd rustploy-gui && cargo test --workspace`) e, se precisar, libere
  espaço em `target/`.
- **Assinatura sem digitar a senha:** o `tools/sign-release.sh` chama `minisign -S` uma vez por arquivo e o `minisign` lê a senha
  da entrada padrão quando não há terminal. Um *shim* temporário no `PATH` que faz `cat minisign.pass | /usr/bin/minisign "$@"`
  só para `-S` assina tudo de uma vez (o `minisign.pass` fica fora do git). Use `--dry-run` antes: ele assina e confere tudo,
  mas não envia nada.
- **`install.ps1` num PowerShell no Linux** precisa de `-e LOCALAPPDATA=/tmp/la -e APPDATA=/tmp/ap -e USERPROFILE=/root` (e as
  pastas criadas); sem isso quebra em `Path` nulo. Vai até o SHA-256 e o download do `minisign.exe`, que não roda no Linux:
  a conferência da assinatura no Windows continua **sem teste real**.
- **Teste do comando de uma linha** pela URL pública em `debian:12` (podman): app (`-- gui`, instalou `rustploy-gui 0.1.35-1`) e
  daemon (`rustployd 0.1.20-1`; sem systemd no contêiner, o serviço não sobe).
- **Textos da interface mudaram de idioma:** testes que comparam texto de tela passam a comparar o **inglês** (o padrão).

## Deploy automático nos servidores (job `deploy-servers`) — DESATIVADO (comentado no workflow)

Depois do job `release` (só em tag), o `release.yml` entra por SSH nos dois servidores do dono
(`itemize@179.198.122.159` e `antonio@chiquitos.tech`) e roda `cd ~/rustploy && git pull && git submodule
update --init --recursive && make install-rustploy`. Precisa de dois segredos em *Settings > Secrets and
variables > Actions*; sem eles o job é **pulado** (a release sai normalmente):

- `DEPLOY_SSH_KEY`: chave privada sem senha, com a pública no `~/.ssh/authorized_keys` dos dois.
- `DEPLOY_KNOWN_HOSTS`: `ssh-keyscan -t ed25519 179.198.122.159 chiquitos.tech`.

Os servidores precisam de `sudo` sem senha para esse usuário (o `make install-rustploy` usa `sudo`
dpkg/systemctl) e compilam o daemon na própria máquina, então o job demora (teto de 90 min). Um servidor
fora do ar não impede o outro (`fail-fast: false`).

## v0.1.58 (2026-11-10): jobs e aquecimentos na fila de deploys

- **O que muda:** daemon **0.1.50**, app **0.1.52** e `rustploy-shared` **0.25.0**. Execução de job (agendada ou "Executar agora") e aquecimento viram itens da fila global (`QueueTask` em `features/deploy/queue.rs`, ids `jrun_`/`jwarm_`), com selo nas três listas do Deploy Engine (app e web UI), promover/reordenar/cancelar pelos botões de sempre (`DeployAbort` aceita os dois ids). `ActiveDeployInfo.kind` com `serde(default)`. Removido `esperar_vaga` (sem uso).
- **Verificado:** shared 53 testes; daemon 554 (os 19 de `web_ui` só passam sozinhos: sob `cargo test --workspace` junto, 8 falharam por carga e passaram isolados, como já registrado na v0.1.56); app `cargo test --bins`, `app_manifest` e `glacier test` (422); `make lint` verde (o clippy pegou um `sort_by`, corrigido; o `fmt` pegou 4 trechos); site 929 páginas, 0 links quebrados; `workflow_dispatch` verde antes da tag; `.deb` com versão certa; release **assinada** (9 arquivos); `sha256sum -c` e `minisign -V` nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` (podman) instalaram `rustployd 0.1.50-1` e `rustploy-gui 0.1.52-1`; docs no ar (pt e en, 200 e texto novo, conferido na URL do worker).
- **NÃO verificado:** um daemon de verdade com um job esperando atrás de um deploy; as telas com o selo (nunca vistas); `DeployAbort` de job ponta a ponta; `install.ps1` num Windows. O servidor dev ainda precisa ser atualizado para a 0.1.50.
- **Fora da fila:** limpeza diária do Docker e aquecimento de serviço (`service_warm`) ainda usam a vaga por fora; contadores de 24h só contam deploys; aquecimento não aparece no Histórico; aquecimento em execução não é abortável.
- **Interrupção:** a sessão caiu durante o `cargo publish`; conferi no crates.io que o daemon já tinha saído e publiquei só o app.
- **Assinatura:** feita por mim com o `minisign.pass`, autorizada de novo pelo dono neste pedido ("publica, pode assinar com o minisign.pass"); shim só para `-S`, `--dry-run` antes, shim apagado.

## v0.1.57 (2026-11-10): acabamento da tela da retenção de tags

- **O que muda:** daemon **0.1.49** e app **0.1.51** (shared 0.24.0 igual). App: o texto do resumo do registry no pt-BR usava `@x` em vez de `{x}`; **TAGS A MANTER** virou spinbox (1 a 1000) em duas linhas, sem ser espremido pelo interruptor; o formulário segue o valor salvo do servidor quando ele muda (antes copiava uma vez só e ficava vazio/desligado); classe `.env__toolbar` (espaçamento 16) nas barras de Volumes, Imagens e Networks. Web UI: o campo virou `<input type="number" min=1 max=1000>` e o Salvar limita a 1000.
- **Como foi achado:** o dono abriu a tela da v0.1.56 e mandou capturas. A 0.1.56 foi publicada sem ninguém ver a tela, e ela tinha o texto cru, o campo espremido e o formulário vazio. **Lição:** o que o `glacier test` não cobre (layout, espaçamento, tradução do pt-BR) só aparece olhando; aqui o dono viu antes de qualquer teste.
- **Verificado:** `make lint` verde; daemon 553 testes (e os 19 de `web_ui` sozinhos); app `cargo test --bins`, `app_manifest` e `glacier test` (421); site 929 páginas, 0 links quebrados; `workflow_dispatch` verde antes da tag; release **assinada pelo dono** (9 arquivos); `sha256sum -c` e `minisign -V` nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo (podman) instalaram `rustployd 0.1.49-1` e `rustploy-gui 0.1.51-1`; docs no ar (pt e en, 200 e texto novo).
- **NÃO verificado:** as telas **depois** desta correção (spinbox alinhado ao Salvar, espaçamento das barras, campo numérico da web UI) não foram vistas; a web UI pode ter nas barras de Volumes/Imagens/Networks o mesmo problema de espaçamento (não foi olhada); a retenção contra um registry real; `install.ps1` num Windows de verdade.
- **Falha do CI que não era do código:** na tag, o job arm64 caiu no `apt` ("File has unexpected size… Mirror sync in progress?", espelho do Ubuntu), o amd64 foi cancelado junto. `gh run rerun <id> --failed` bastou; a tag não foi mexida porque a release ainda não existia.
- **Assinatura:** feita pelo dono (a autorização do `minisign.pass` da v0.1.56 valeu só para aquele publish; perguntei antes).

## v0.1.56 (2026-11-10): retenção de tags do registry embutido

- **O que muda:** daemon **0.1.48**, app **0.1.50** e `rustploy-shared` **0.24.0**. `features/registry/retention.rs`: antes de cada GC (o job diário e o botão), com a política ligada, apaga em cada repositório as tags além das `keep_last` mais recentes e **nunca** a que um serviço de fonte Registry usa (`push_deploy::tags_in_use`). Só a linha da tag (`delete_tag_only`). Guardada em `settings.yml` (`registry_retention`). `RegistryRetentionSet` (Admin) e `RegistryStatusInfo.retention`. Interruptor e "TAGS A MANTER" em Docker → Registry (app e web UI). Desligada por padrão.
- **Por que:** o GC só apaga o que nenhuma tag aponta, e o workflow do `flow-client` publica uma tag por commit (`<sha>`), que acumulava uma imagem por push.
- **Achado no caminho:** o `RegistryTagDelete` (botão de remover tag) apaga **por digest**: remove também toda tag que aponte para o mesmo manifest. Um rebuild idêntico dá o mesmo digest a duas tags, então apagar a tag do SHA levaria a `staging` junto. A retenção por isso apaga só a linha da tag. O comportamento do botão foi mantido (está documentado), mas é uma armadilha.
- **Verificado:** shared 51 testes; daemon 553 (7 novos: política pura, banco com manifests de mesmo digest, `tags_in_use`, YAML); app 421 no `glacier test` (+1), `cargo test --bins` e `app_manifest`; `make lint` verde; site 929 páginas, 0 links quebrados; `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos); `sha256sum -c` e `minisign -V` nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo (podman) instalaram `rustployd 0.1.48-1` e `rustploy-gui 0.1.50-1`; docs no ar (pt e en, 200 e texto novo).
- **NÃO verificado:** a retenção contra um registry de verdade (só SQLite de teste), nem o GC logo depois liberando o disco; as telas novas (app e web UI) nunca foram vistas funcionando: só `glacier test` e o teste headless que renderiza as telas principais; se o interruptor da web UI mantém o que o usuário digita entre dois snapshots; `install.ps1` num Windows de verdade; o servidor dev ainda precisa ser atualizado para a 0.1.48.
- **Teste instável sob carga:** dois testes headless da web UI (`all_main_screens_render`, `deploy_plan_modal_lists_order_and_buttons`) falharam uma vez quando rodaram ao mesmo tempo que lint e testes da GUI (40 s contra 17 s) e passaram sozinhos, duas vezes. **Rode a suíte do daemon sozinha.**
- **Assinatura sem o dono:** o dono pediu que eu assinasse com a senha do `minisign.pass`; usei o shim do `PATH` descrito acima (só para `-S`, apagado depois), com `--dry-run` antes.

## v0.1.55 (2026-11-10): a rota do domínio não some num deploy; aba Registry com selects

- **O que muda:** daemon **0.1.47** e app **0.1.49** (shared 0.23.0 igual). `features/deploy/recovery.rs`: o reconcile de 30 s pula serviços com deploy em andamento (`services_with_deploy_in_flight`); `executor/promote.rs`: o `Promoting` registra a rota de novo depois do rename. App e web UI: sub-aba **Registry** em General, com selects de repositório e tag (`RegistryTagList`), imagem montada como `127.0.0.1:<porta>/repo:tag`.
- **Como foi achado:** no servidor dev, depois de converter o `ui` para Registry e fazer o deploy (Live), `dev.itemize.com.br` deu 404 e a rota sumiu da tabela do ingress; `ingress_reconcile` a trouxe de volta. Pelo código: entre o `SwappingIn` (para o container antigo) e o `Promoting` (renomeia o novo) não há container live com IP, e o reconcile lia isso como serviço caído. **Lição:** rotina periódica que "corrige" estado precisa ignorar o que um deploy está mudando.
- **Verificado:** teste novo do daemon (`in_flight_tests`); daemon 562 testes, app `cargo test --bins` 30 + `app_manifest` 7 + `glacier test` 420, shared 48; `make lint` verde; site 929 páginas, 0 links quebrados; `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos); `sha256sum -c` e `minisign -V` (chave do repositório) nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo (podman) instalaram `rustployd 0.1.47-1` e `rustploy-gui 0.1.49-1`; docs no ar (pt e en, 200 e texto novo).
- **NÃO verificado:** a causa da rota **não** foi confirmada em log do daemon (dedução do código e do sintoma); o conserto não foi exercitado num deploy real (o teste novo só cobre a seleção dos serviços em andamento, não a corrida); a aba Registry nunca foi vista na tela, nem no app nem na web UI (só `glacier test`, `node --check` e leitura do código); `install.ps1` num Windows de verdade; o push-to-deploy disparado por um push real do CI.
- **Para conferir depois de atualizar o servidor dev:** um deploy do `ui` e ver se `ingress_routes` mantém `dev.itemize.com.br`.

## v0.1.54 (2026-11-09): também os tokens de login do provider Git

- **O que muda:** daemon **0.1.46** (app 0.1.48 e shared 0.23.0 iguais). `seal_provider_tokens` (em `store/infra/seal.rs`) cifra no lugar o `access_token_enc` e o `refresh_token_enc` em claro da tabela `git_provider_token`, sem mudar o valor.
- **Como foi achado:** a v0.1.53 zerou os "valores em claro", mas o **Salvar e aplicar** do Sync falhou com `StateGitFailed: valor não está no formato ENC[age:…]` (`configure` → `refuse_unrelated_history` → `usable_token` → `decrypt`). **Lição:** o `InfraAudit` só vê o YAML; segredo que mora no SQLite não aparece nele. Ao cifrar legado, siga **todos** os caminhos que chamam `decrypt`, não só os que o audit lista.
- **Verificado:** 2 testes novos (cifra sem mudar o valor, só access, já cifrado intacto, idempotência); daemon 561 testes + clippy + fmt; site com 0 links quebrados; `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos); `sha256sum -c` e `minisign -V` (chave da máquina 1) nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo (podman) instalaram `rustployd 0.1.46-1` e `rustploy-gui 0.1.48-1`; `install.ps1` no Linux chegou até o SHA-256 (para no `minisign.exe`, como esperado); docs no ar (pt e en, 200 e texto novo).
- **NÃO verificado:** a migração contra o SQLite real do servidor dev (só dados sintéticos); ligar o sync de fato; outros valores em claro fora do YAML e de `git_provider_token` (só revisei os `decrypt` do caminho do sync); `install.ps1` num Windows de verdade.

## v0.1.53 (2026-11-09): o daemon cifra no boot o que a migração deixou em claro

- **O que muda:** daemon **0.1.45** (app 0.1.48 e shared 0.23.0 iguais). `store/infra/seal.rs` cifra no lugar o token de webhook e o `client_secret` de provider que estavam em claro, sem mudar o valor; roda no boot, é idempotente e só avisa se falhar. Motivo: o `InfraAudit` os trata como certeza e o sync com o git recusava ligar no servidor dev (6 achados: 5 webhooks do projeto Flow e o provider GitHub).
- **Verificado:** 4 testes novos (cifra sem mudar o valor, idempotência, infra vazia, provider sem secret); daemon 559 testes + clippy + fmt; site com 0 links quebrados; `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos); `sha256sum -c` e `minisign -V` com a chave da máquina 1 nos 4 pacotes; `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo (podman) instalaram `rustployd 0.1.45-1` e `rustploy-gui 0.1.48-1`; `install.ps1` num PowerShell no Linux (com `LOCALAPPDATA`/`APPDATA`/`USERPROFILE`) chegou até o SHA-256 e parou no `minisign.exe`, como esperado; docs no ar (pt e en com 200 e o texto novo).
- **NÃO verificado:** a migração contra os dados reais do servidor dev (só testes que reproduzem o estado legado; falta atualizar o servidor e rodar o `InfraAudit`); ligar o sync de fato; `make lint`/`make test` do gui e do shared (não mudaram); `install.ps1` num Windows de verdade.
- **Nota de ambiente:** nesta máquina o `docker` não existe, só o `podman`; os testes de instalação rodaram com `podman run docker.io/library/debian:12`.

## v0.1.52 (2026-11-09): correção da importação de dados no Podman

- **Histórico do nome:** uma primeira tentativa de tag `v0.1.52` foi cancelada pelo dono depois do CI verde, antes de assinar (tag e rascunho apagados). O nome foi reaproveitado, a pedido do dono, para a release de fato: daemon 0.1.44 e app 0.1.48 já estavam no crates.io (sem *yank*) e na `main`.

- **O que muda:** daemon **0.1.44** e app **0.1.48** (shared 0.23.0 igual). App: `.reorder__index` no lugar de `.env__key` (width 400) na janela Reordenar serviços; os nomes sumiam. `features/service_data/stream.rs` copia stdin/stdout por pipe; Postgres recria o database no restore.
- **Como foi achado:** depois de publicar a v0.1.51 descobri que **havia Podman na máquina** (meu `which docker podman` voltou vazio por causa do filtro do terminal) e rodei os testes
  `#[ignore]` contra ele (`RP_TEST_SOCKET=/run/user/1000/podman/podman.sock`): `volume_vai_e_volta_pelo_tar` falhou (`tar: short read`) e `postgres_dump_e_restore_de_verdade` mostrou a tabela que sobrava.
  **Lição:** antes de afirmar "sem motor de containers", cheque `command -v podman` por fora do filtro, e rode os `#[ignore]` ANTES de publicar.
- **Verificado:** os dois testes de integração passam no Podman rootless; daemon 538 testes + clippy.
- **Release:** `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos: 4 pacotes, 4 `.minisig`, `checksums.sha256`); `install.sh` e `install.sh -- gui` pela URL pública num `debian:12` limpo instalaram `rustployd 0.1.44-1` e `rustploy-gui 0.1.48-1`; app testado num worktree limpo (`cargo test`, clippy, `glacier test` 417); docs no ar (pt e en, 0 links quebrados).
- **NÃO verificado:** a janela Reordenar serviços rodando depois do conserto (só `glacier test`, `cargo test` e lint); MySQL, MariaDB e MongoDB reais; falha no meio da importação; export→import cifrado; as abas na tela; Docker (só Podman rootless foi usado).

## v0.1.51 (2026-11-09): exportar e importar os dados de um serviço

- **O que muda:** daemon **0.1.43**, app **0.1.47**, `rustploy-shared` **0.23.0**. `features/service_data` (partes, pacote `.rpdata`, dump de banco, transferência com rede de segurança),
  rotas `/data/export` e `/data/import` (só chave mestra), cifra `age` de arquivo em `platform/age_format.rs`, aba **Dados** na web UI e no app, guia `guias/dados-do-servico` (pt e en).
- **crates.io:** `rustploy-shared` 0.23.0, `rustploy` 0.1.43 e `rustploy-gui` 0.1.47 (2026-11-09). O app foi publicado de um **worktree limpo**: o `rustploy-gui` tinha alterações
  de outra frente na árvore de trabalho (janelas renomeadas para `*.screen.gv`, `app.gv`, `app.gss`) que **não** entraram.
- **Verificado:** shared 48 testes e clippy; daemon 538 testes e `clippy -D warnings`; app (worktree limpo): `cargo test`, clippy e `glacier test` (um teste meu de ordem alfabética de `busy_actions` falhou e foi corrigido);
  site com 0 links quebrados.
- **NÃO verificado:** nada disto rodou contra um motor de containers (sem Docker/Podman na máquina de desenvolvimento): o teste `volume_vai_e_volta_pelo_tar` (`#[ignore]`) nunca rodou, nem dump/restore
  de Postgres/MySQL/Mongo reais, nem a falha no meio da importação, nem o ciclo cifrado export→import, nem as duas abas na tela.

## v0.1.50 (2026-11-08): reordenar os serviços de um projeto

- **O que muda:** daemon **0.1.42**, app **0.1.46**, `rustploy-shared` **0.22.0**. Comando `ProjectServicesReorder`; ordem em `ProjectInfra.service_order` (`project.yml`),
  `services_in_order()` e poda no `normalize`; `store::services::reorder`; modal na web UI e janela própria no app (`services_reorder_window`). Docs pt e en.
- **crates.io:** `rustploy-shared` 0.22.0, `rustploy` 0.1.42 e `rustploy-gui` 0.1.46 (2026-11-08).
- **Verificado:** shared 48 testes; daemon 525 testes e `clippy -D warnings` (dois do store, um headless do modal); app: clippy, `cargo test` e `glacier test` 416; site com 0 links quebrados.
- **Release:** `workflow_dispatch` verde antes da tag; release **assinada** (9 arquivos, checksums e 4 `.minisig` conferidos); `install.sh gui` pela URL pública num `debian:12` limpo instalou `rustploy-gui 0.1.46-1`; docs no ar (pt e en) com 200 e o conteúdo novo.
- **NÃO verificado:** a janela do app e o arraste com o mouse; o arraste da web UI num navegador (o teste move por código); o comando contra um daemon real e o ida-e-volta do `project.yml` pelo git.

## v0.1.49 (2026-11-08): Deploy com dependências e Deploy do projeto

- **O que muda:** daemon **0.1.41**, app **0.1.45**, `rustploy-shared` **0.21.0**. `features/deploy/tree.rs` (planejamento por ondas, `run_plan`), espera por dependência em
  `queue::run_one`, validação de dependência impossível em `check_depends_on` e no `SharedAccessGrant`. Comandos `DeployWithDependencies`, `ProjectDeploy` e os dois `*Plan`;
  modal da ordem no app (`prompt` de três opções) e na web UI (três botões); botão Deploy no cabeçalho do projeto. Docs pt e en (guia de dependências, telas de serviço e projeto,
  referência gerada).
- **crates.io:** `rustploy-shared` 0.21.0, `rustploy` 0.1.41 e `rustploy-gui` 0.1.45 (2026-11-08).
- **Verificado:** shared 48 testes; daemon 522 testes e `clippy -D warnings` (seis testes de integração do lote, cenários de ordem, pulo por falha e ciclo); web UI com Chrome
  headless (modal com ordem e botões); `glacier test` 413; site com 0 links quebrados.
  `workflow_dispatch` antes da tag (todos os jobs verdes, `.deb` amd64/arm64 e app com as versões certas); release **assinada** (9 arquivos, checksums e 4 `.minisig` conferidos com
  `keys/rustploy-update.pub`); `install.sh gui` pela URL pública num `debian:12` limpo (podman) instalou `rustploy-gui 0.1.45-1`; docs no ar (pt e en) respondendo 200 com o conteúdo novo.
- **Achado:** o `cargo publish --dry-run` do daemon falhou com `rustc` saindo 101 por **disco cheio** (o target do daemon passa de dezenas de GB); liberar espaço e repetir resolveu. O teste headless
  do modal precisou de uma espera de 300 ms após mudar o plano (o `x-show` do Alpine reavalia de forma assíncrona).- **NÃO verificado:** o fluxo **num servidor real** (deploy de um projeto com banco + api + worker, a fila na ordem, o pulo quando o banco falha); o modal do app visto na tela
  (só `glacier test`, sem janela); o app contra um daemon antigo.

## v0.1.48 (2026-11-08): chave mestra e de visitante, com níveis

- **O que muda:** só o daemon, **0.1.40** (app 0.1.44 e `rustploy-shared` 0.20.0 inalterados). `[api].master_token` e `[api].visitor_level`; com a mestra, a chave atual vira
  a de visitante e passa pelas proibições, pelo nível, pela restauração de máscaras e pela redação em **qualquer** rota (`/mcp` e `/api/rpc`). Níveis `observer`/`operator`/`admin`
  (`api/agent/policy.rs`, `match` exaustivo sobre os 127 comandos); `rustployd access-key --master`; `JobUpdate` restaura variáveis mascaradas; `InfraAuditAllow` proibido ao agente;
  `/api/events` e o upload de zip recusados ao visitante. Docs pt e en (`instalacao/config-toml`, `agentes/seguranca`) e manual do agente. Plano: `docs/plano-limites-do-agente-mcp.md`.
- **crates.io:** `rustploy` 0.1.40 (2026-11-08).
- **Verificado:** daemon 505 testes e `clippy -D warnings`; a política por testes automatizados (chamador por chave, proibições e níveis pelo mesmo caminho do `/api/rpc`, senha mascarada
  para o visitante e visível para a mestra, `ServiceUpdate` e `JobUpdate` do operador recusados ao mudar volumes/comando/compose, geração e gravação da mestra no `config.toml`);
  site com 0 links quebrados; `workflow_dispatch` antes da tag (todos os jobs verdes); release **assinada** (5 arquivos + 4 `.minisig`, checksums e assinaturas conferidos com
  `keys/rustploy-update.pub`); `install.sh gui` pela URL pública num `debian:12` limpo (via podman) instalou o app `0.1.44-1`; docs no ar (pt e en) com as páginas de segurança e do
  `config.toml` respondendo 200.
- **NÃO verificado:** o fluxo inteiro **num servidor real** (gerar a mestra, reiniciar, entrar na GUI/webui com a mestra, agente com a chave antiga); a GUI e a webui com a chave de
  visitante (o que a pessoa vê ao ser recusada); o `access-key --master` contra um `config.toml` de produção. As decisões sobre níveis (três níveis, `operator` padrão; `state_git` e criar
  serviços em `admin`; o operador liga/desliga jobs) foram tomadas com as sugestões do plano e **confirmadas pelo dono em 2026-11-08**.

## v0.1.47 (2026-11-07): a credencial do provider git só vai para o servidor do provider

- **O que muda:** só o daemon, **0.1.39** (app 0.1.44 e `rustploy-shared` 0.20.0 inalterados). `features/git_providers::{https_authority, provider_authority,
  credential_matches_url, usable_token_for_url}`, usadas por `resolve_clone_credentials` (deploy, job e aquecimento) e pelo `state_git` (recusa o `remote` na
  configuração). O `match` de nomes do `dispatch` ficou sem braço curinga. Nota nas páginas `guias/git` (pt e en).
- **crates.io:** `rustploy` 0.1.39 (2026-11-07).
- **Verificado:** daemon 491 testes e `clippy -D warnings`; a conferência de host por testes unitários (`github.com.atacante.com`, `github.com@atacante.com`,
  porta e caixa diferentes, Enterprise, Gitea); site com 0 links quebrados; `workflow_dispatch` antes da tag (todos os jobs verdes); release **assinada**
  (5 arquivos + 4 `.minisig`, checksums e assinaturas conferidos com `keys/rustploy-update.pub`); `install.sh gui` pela URL pública num `debian:12` limpo
  (via podman) instalou o app `0.1.44-1`; docs no ar (pt e en) com a nota nova respondendo 200.
- **NÃO verificado:** um clone real de repositório privado com o provider (o caminho com rede e com um provider de verdade não tem teste automatizado);
  o `state_git` contra um GitHub real depois da mudança. Um provider com **BASE URL** diferente do host real deixa de autenticar (esperado, documentado).

## v0.1.46 (2026-11-07): `reconcile` não condena o `Live` no boot

- **O que muda:** só o daemon, **0.1.38** (app 0.1.44 e `rustploy-shared` 0.20.0 inalterados); `glacier-ui` 0.152.1. O `reconcile` deixa de marcar `Stopped` (e de
  apagar o `live_container_id`) o deploy `Live` de um serviço com intenção "no ar" nos primeiros 10 min do daemon, e volta a `Live` o deploy mais recente
  que ele mesmo condenou quando o container está rodando. Detalhes no `CHANGELOG.md`.
- **crates.io:** `rustploy` 0.1.38 (2026-11-07); `glacier-ui` 0.152.1.
- **Verificado:** daemon 487 testes e `clippy -D warnings` (as duas regras do `reconcile` por testes unitários puros); `glacier-ui` com a suíte completa
  (`--features suite-completa`) passando.
  `workflow_dispatch` antes da tag (todos os jobs verdes); release **assinada** (5 arquivos + 4 `.minisig`, checksums e assinaturas conferidos com
  `keys/rustploy-update.pub`); `install.sh gui` pela URL pública num `debian:12` limpo (via podman) instalou o app `0.1.44-1`. Docs: nenhuma página mudou, então
  não houve deploy do site.
- **NÃO verificado:** o caminho do `reconcile` com o Docker (não há teste automatizado) — falta confirmar no Chiquitos que o deploy da api e o do worker
  voltam a `Live` e que um reinício do daemon não os derruba; `install.sh` do daemon e `install.ps1`.

## v0.1.45 (2026-11-07): edição de variável no lugar, push-to-deploy, `EnvValueSet`

- **O que muda:** daemon **0.1.37**, app **0.1.44**, `rustploy-shared` **0.20.0** (aditivo: `Command::EnvValueSet`, `ServiceSpec.registry_auto_deploy`,
  `Event::RegistryPush`); `glacier-ui` 0.151.0 (campo rola na horizontal, toast animado) e 0.152.0 (`textinput(autofocus)`). Edição de variável no
  lugar (✏️ chave, ✎ valor) no app e na web UI; push-to-deploy do registry embutido (opt-in na aba Advanced); `/api/rpc` recusa `ServiceUpdate`
  parcial; correção do boot de serviço sem healthcheck que não escuta porta; `sw.js` busca o shell sem o cache HTTP; manual do agente com os
  comandos que faltavam (teste de cobertura). Detalhes no `CHANGELOG.md`.
- **crates.io:** `rustploy-shared` 0.20.0, `rustploy` 0.1.37 e `rustploy-gui` 0.1.44 (2026-11-07); `glacier-ui` 0.151.0 e 0.152.0.
- **Verificado:** daemon 485 testes e `clippy -D warnings`; shared 47 testes; app 413 testes Luau e os de Rust; no `glacier-ui`: o campo rolando,
  a barra do toast, a entrada/saída e o `autofocus` num exemplo sob Xvfb + kwin; `EnvValueSet`, o casamento do push-to-deploy e o aviso do PUT de
  manifest por testes automatizados; site com 0 links quebrados; `workflow_dispatch` antes da tag (todos os jobs verdes); release **assinada**
  (5 arquivos + 4 `.minisig`, checksums e assinaturas conferidos com `keys/rustploy-update.pub`); `install.sh gui` pela URL pública num
  `debian:12` limpo (via podman) instalou `rustploy-gui 0.1.44-1`; docs no ar (pt e en) com as páginas novas respondendo 200.
- **Armadilha nova:** `cargo fmt -- <arquivo>` formata o **workspace inteiro** (segue os `mod`/o crate), não só o arquivo; no `glacier-ui`, onde o código
  não é `fmt`-limpo, isso enche o diff de ruído. Formate com `rustfmt <arquivo>` num arquivo sem `mod`, ou só rode `cargo fmt` onde o `--check` já passa.
- **NÃO verificado:** a edição no lugar **rodando** no app e na web UI (só testes); o app com o campo que rola e o toast novos; o push-to-deploy num
  servidor real (`docker login` → `docker push` → redeploy); o boot do Chiquitos sem o erro de 120s do worker; `ServiceBundle` não leva
  `registry_auto_deploy`; o `install.sh` do daemon (precisa de systemd) e o `install.ps1` num Windows de verdade. O `glacier-ui` ainda tem 18
  testes de lib falhando que já existiam (parser, luau, canvas, charts, eval) — **corrigidos depois, em 2026-11-07** (ver o `TODO.md`).

## v0.1.44 (2026-11-06): logs por execução

- **O que muda:** daemon **0.1.36**, app **0.1.43**, `rustploy-shared` **0.19.0** (aditivo). O log de runtime passa a ter uma pasta por
  execução (container) com `meta.json`; a aba Logs mostra só a execução atual ao vivo e uma lista de **Execuções anteriores** (Ver logs numa
  janela com data e hora, Excluir, Limpar selecionados). `LogsHistory` ganha `container_ids`; novos `LogsExecutions` e `LogsExecutionDelete`.
  A retenção apaga execuções inteiras. No primeiro boot o daemon **migra** os segmentos da v0.1.40 para as pastas novas (execuções antigas ficam
  "sem deploy associado"). No app: título do serviço editável (grava sozinho, com debounce), botões de mover/adicionar travados sem destino,
  espaçamento nos botões das linhas de variável; `glacier-ui` 0.150.1 (`size`/`bold` no `textinput`). Detalhes: `docs/plano-log-de-runtime-persistido.md` §7.
- **crates.io:** `rustploy-shared` 0.19.0, `rustploy` 0.1.36 e `rustploy-gui` 0.1.43 (2026-11-06); `glacier-ui` 0.150.1.
- **Verificado:** daemon 477 testes e `clippy -D warnings`; `LogsExecutions`/`LogsHistory`/`LogsExecutionDelete` e a migração contra um daemon
  local (engine falso); a webui em Chrome headless; 404 testes Luau; `workflow_dispatch` antes da tag; release assinada; `install.sh gui` num
  `debian:12` limpo (app 0.1.43-1); docs no ar (pt e en), 0 links quebrados.
- **NÃO verificado:** a janela nativa da GUI com a lista (o app do dono estava aberto no teste); a recusa da execução **atual** com um container
  `live` real; a migração numa cópia dos logs do Chiquitos; `install.sh` do daemon (precisa de systemd) e `install.ps1`.

## v0.1.43 (2026-11-06): seletor de idioma do app

- **O que muda:** só o app, **0.1.42**. Os botões English / Português saem da barra do topo: no login ficam num rodapé discreto no canto
  inferior direito; dentro do app, numa aba **Idioma** em Configurações. A web UI não mudou. Daemon **0.1.35** e `rustploy-shared`
  **0.18.0** não mudam.
- **crates.io:** `rustploy-gui` 0.1.42.
- **Verificado:** `cargo fmt`, `clippy -D warnings`, 40 testes; a tela foi aberta e testada pelo dono antes do publish. No Chiquitos (daemon
  0.1.35, publisher novo do chickie) os `missed heartbeats` do RabbitMQ sumiram: a `api` ficou 43 min conectada.
- **NÃO verificado:** a reconexão do publisher depois de uma queda do broker em produção (só o teste automático). O falso `Stopped` do
  primeiro deploy depois do restart do daemon (`container not running at read time`, 06:24) não tem causa conhecida e não repetiu.

## v0.1.42 (2026-11-06): `product` no lugar de `db_kind`, webhook form, IP fixo

- **O que muda:** daemon **0.1.35** + `rustploy-shared` **0.18.0** (o app 0.1.41 já usa o `product`). `ServiceSpec.db_kind` virou
  `product` (banco, broker ou storage); o daemon converte o YAML/SQL antigo na leitura (infra schema 2, com backup `infra.bak-schema1`)
  e o manifesto do pacote aceita `db:` como alias. `/api/rpc` recusa `ServiceCreate/Update` com `db_kind` (cliente antigo).
  Webhook aceita `application/x-www-form-urlencoded` e ignora o ping. `attach_network_alias` reconecta com o **mesmo IPv4**
  (sem isso o IP mudava e conexões abertas morriam por heartbeat 2-3 min depois do deploy). Deploy em fila preserva `live_container_id`.
- **crates.io:** `rustploy-shared` 0.18.0 e `rustploy` 0.1.35 publicados (2026-11-06).
- **Verificado:** `make lint` e `make test` (485 testes do daemon); `alias_mantem_o_ip` em Podman rootful 5.7.0, com e sem o 2º
  container (sem o conserto falha); site: build e 0 links quebrados.
- **Verificado depois (2026-11-06, pelo dono):** daemon atualizado pelo botão do updater; conversão `db_kind` -> `product` num servidor
  com dados reais, sem problemas.
- **NÃO verificado:** IP fixo e fim dos `missed heartbeats` no Chiquitos.

## v0.1.41 (2026-11-05): fix da janela de logs do app

- **O que muda:** só o app, **0.1.41**. A janela de logs da v0.1.40 dava "attempt to call a nil value" ao abrir **Logs**:
  `format.history_rows` não estava exportado pela fachada `views/scripts/format.luau`. Também o separador de container do log
  passa a sair em pt-BR no catálogo pt-BR. Daemon **0.1.34** e `rustploy-shared` **0.17.0** não mudam.
- **crates.io:** `rustploy-gui` 0.1.41 publicado (2026-11-05).
- **Verificado:** `cargo fmt`, `clippy -D warnings`, 40 testes e `cargo publish --dry-run`; a GUI instalada com `make install-gui`
  abre a janela de logs da `api` do Chiquitos e mostra o histórico (11 linhas).
- **NÃO verificado:** o botão **Carregar anteriores** (só aparece com mais de 500 linhas guardadas num serviço que não seja Compose).
- **Armadilha:** função nova em `format/*.luau` precisa entrar também na fachada `views/scripts/format.luau`; os testes Luau não pegam a falta.

## v0.1.40 (2026-11-05): Baixar a infra e log de runtime persistido

- **O que muda:** daemon **0.1.34**, app **0.1.40**, `rustploy-shared` **0.17.0** (**quebra de protocolo**).
  (1) **Baixar a infra:** `InfraDownload` devolve os arquivos que o `state_git` grava (secrets `ENC[age:…]`; recusa com
  `PlaintextInInfra` se o git também recusaria); botão na aba Sync da GUI e da webui. Saem a aba **Infra as Code** e os comandos
  `ManifestApply/Export/ExportAll/Import`. (2) **Log de runtime persistido:** arquivos por serviço em `<dados>/logs/`, 14 dias / 50 MiB
  (`[logs]`), captura antes de remover o container (troca e rollback), comando `LogsHistory` e "Carregar anteriores" nas telas.
  (3) Settings carrega as contas Git sozinho; fix do hover dos links. **GUI/webui antigas contra este daemon** perdem a aba IaC
  (comandos removidos): atualizar os dois juntos.
- **crates.io:** `rustploy-shared` 0.17.0, `rustploy` 0.1.34 e `rustploy-gui` 0.1.40 publicados; o daemon **não usa mais `path`** para o shared
  (a pendência de 2026-11-03 acabou). `cargo install rustploy` volta a refletir a `main`.
- **Verificado:** `make lint`/`make test` (daemon 476), `glacier test` 399, webui headless 16, site 927 páginas e 0 links quebrados, CI da tag,
  comando de uma linha do `install.sh` num `debian:12` limpo (daemon 0.1.34 e app 0.1.40, com a assinatura conferida), páginas novas do site com 200,
  `InfraDownload` num daemon local (4 arquivos; recusa com senha em claro) e o log persistido num daemon local com Podman (rollback, stream
  contínuo, reinício do daemon sem duplicata, paginação).
- **NÃO verificado:** a captura do container antigo na troca (`Promoting`) num engine real (o Podman rootless local recusa o alias do
  `SwappingIn`; o rollback usa a mesma função e funcionou); a janela de logs da **GUI nativa**; o `install.ps1`; os servidores (Chiquitos, itemize) ainda no daemon anterior.
- **Armadilhas:** (a) o **Podman entrega timestamps de log com resolução de segundo**: a deduplicação do `log_store` precisa de instantes únicos
  (`Stamper` em `container_logs.rs`), senão descarta as linhas seguintes do mesmo segundo; (b) `pkill -f` casa com a própria linha de comando
  do shell que o chama: matar por PID exato (e nunca o `rustployd` do sistema); (c) `title="…\"x\""` em `<Aside>` do MDX quebra o build: sem aspas no título.

## v0.1.39 (2026-11-05): webhook filtra por `watch_paths`

- **O que muda:** o webhook de deploy (`POST /webhook/<id>/<token>`) passa a respeitar `watch_paths` do serviço Git: com o campo
  preenchido, só dispara se algum arquivo do push (`commits[].added|modified|removed`) casar; senão responde
  `200 skipped: no watched path changed`. Daemon **0.1.33**; o `rustploy-shared` (0.16.0) e o app (0.1.39) **não mudaram**.
- **Na dúvida faz o deploy** (nunca perde um push): corpo sem a lista, `commits` vazio, commit sem alguma das listas, merge sem arquivos,
  10+ commits (GitHub trunca em 20, Gitea em 10). Deploy manual e o do MCP ignoram o filtro. Caminhos relativos à raiz do repositório;
  arquivo, diretório ou glob simples (`*`, `**`).
- **Verificado:** `make lint` e `make test` (462 testes do daemon, 10 deles do filtro) e um **daemon local real** com 8 payloads.
  **Não verificado:** com um GitHub de verdade (o formato do payload vem da documentação dos provedores, não de um push real).
- **Para que serve:** várias aplicações na mesma branch (chickie: aposentar as branches `main-*` e o `make sync`, Fase H do
  `plano-credenciais-cifradas-e-storage.md`). Configurar os apps só **depois** desta release instalada no servidor.
- O `table_link.gv` do app (correção do hover) estava só na árvore de trabalho e **não** entrou nesta release.
- **Release publicada e conferida (2026-11-05):** `v0.1.39` com os 5 arquivos e 4 `.minisig`. O comando de uma linha do README, pela URL
  pública, num `debian:12` limpo (Podman): daemon `0.1.33-1` e app `0.1.39-1` instalaram. O binário do `.deb` reporta 0.1.33 e contém o filtro.
  **Não testado:** `install.ps1` no Windows (não mudou) e o daemon rodando sob systemd (o contêiner não tem). Docs publicadas depois da release.
- **Armadilhas desta vez:** (1) `tools/sign-release.sh` logo depois do `git push` da tag responde "release não encontrada": o rascunho só existe quando
  o job `release` termina (~20 min); espere o CI. (2) `gh run download` precisa rodar **dentro** de um clone (`-R` ou `cd`), senão não baixa nada.
  (3) `make lint` roda um clippy mais estrito que `cargo clippy -p rustploy`: pega `vec!` desnecessário em teste; rode o `make lint` inteiro antes do bump.

## v0.1.38 (2026-11-04): Podman + ufw (DNS dos serviços e saída dos containers)

> As tags `v0.1.36` (daemon 0.1.30, manual com uma gambiarra de rotação de senha) e `v0.1.37` (daemon 0.1.31, auxiliar do ufw com um bug de
> ordem de regras achado pelo laboratório) **ficaram no repositório porque apagar tag remota foi bloqueado**. A execução de ambas foi cancelada;
> se algum rascunho aparecer, **descarte-o**. Vale a `v0.1.38` (daemon 0.1.32).

- **O problema** (achado ao subir o Chiquitos, Podman rootful + ufw `deny incoming/deny routed`): os nomes `rp_*` não resolvem
  dentro dos containers e nada sai para a internet, enquanto o IP direto funciona. O `aardvark-dns` escuta no endereço do host da
  bridge (`10.89.0.1:53`, **não** em loopback). Com Docker não acontece. O `plano-backend-podman.md` já listava "interação com
  ufw" como sem teste.
- **A correção:** `rustploy-daemon/packaging/rustploy-podman-ufw` (em `/usr/libexec/`), chamado pelo `postinst` (motor Podman +
  ufw ativo) e pelo `install.sh --engine podman`. Detecta as faixas (redes `rp_net_*` + `default_subnet_pools`, senão `10.89.0.0/16`),
  libera DNS e saída e **isola as redes entre si** (o `deny` antes do `allow`). O `rustployd-fw` **não muda** (as portas externas são
  um proxy TCP do próprio daemon, tráfego de entrada no host).
- **Validado de verdade, não só com `ufw` de mentira:** num servidor real (DNS por nome, saída e portas internas com o isolamento ativo)
  e num **laboratório** (`rustploy-daemon/crates/daemon/tests/e2e-podman-ufw/run.sh`: contêiner Podman rootless privilegiado, `ufw` 0.36 e
  `iptables-nft` reais, duas bridges + "internet" em namespaces; ~1 min, sem root). O laboratório achou um **bug**: com o `allow` de saída já
  presente, um `deny` acrescentado depois cai atrás dele e perde o efeito (a contagem de regras não mostrava). O auxiliar agora confere a ordem
  e refaz as regras de rota com o `deny` no topo (`ufw route insert 1`).
- **`ServiceReload` não relê secrets** (só para e inicia o mesmo container): trocar um secret exige **redeploy**.
- **Rotação de senha:** feita no Chiquitos com um serviço descartável como gerador — **gambiarra, rejeitada pelo dono**; o recurso
  certo (`SecretGenerate`/`SecretRotate`) está projetado em `docs/plano-geracao-e-rotacao-de-segredos.md`. O manual e as docs
  **não** recomendam mais aquela receita. A `v0.1.36` tem o texto antigo: **não assine o rascunho**; use a v0.1.37.

## v0.1.35 (2026-11-03): credenciais cifradas, renomear e storage — daemon por `path`

- **(RESOLVIDO em 2026-11-05: o `rustploy-shared` 0.16.0 foi ao crates.io e o `path` saiu; o texto abaixo é histórico.)** **O daemon referencia o `rustploy-shared` por `path`** (`rustploy-daemon/crates/daemon/Cargo.toml`, `version` + `path`),
  por escolha do dono, e o `shared` 0.16.0 **não foi ao crates.io**. Consequências: (1) o CI e a release compilam, porque
  o `checkout` do agregador é `submodules: recursive` e o `path` aponta para o `rustploy-shared` ao lado; (2) o
  **`rustploy-daemon` sozinho não compila** (clone só dele não tem o irmão); (3) **`cargo install rustploy` entrega a
  0.1.28**, a última publicada. Para voltar ao normal: `cargo publish` do `rustploy-shared`, tirar o `path` da linha do
  `shared` e `cargo update -p rustploy-shared`.
- **O comando `cargo publish` não foi usado**; a release é só a tag no agregador (rascunho do CI) e a **assinatura**
  (`tools/sign-release.sh`) é com a chave do dono: pede a senha a cada arquivo, então o agente não a faz.
- **Verificação da GUI no Xvfb** (receita em `docs/…`/memória): o `token` em `app.json` é o token **cru** da API, **não** a
  chave em base64; um "‹ Voltar" no formulário já volta ao passo de tipos (dois fecham a janela); com o projeto vazio os
  botões do cabeçalho mudam de lugar. Achado de verdade por essa verificação: o ✏️ do serviço tinha caído na linha
  "arrastando" da lista e não aparecia.
- `minio/minio` e `minio/mc` não são mais puxáveis (Docker Hub e Quay). O wizard usa `cgr.dev/chainguard/minio`.
- **Depois de publicada (2026-11-03):** o comando de uma linha do daemon e o do app foram rodados **pela URL pública** num
  `debian:12` (podman) e instalaram o daemon 0.1.29 e a gui 0.1.39; o `install.ps1` num PowerShell no Linux achou a
  release, baixou o zip e **conferiu o SHA-256**, e parou onde esperado (o `minisign.exe` não roda no Linux; precisa de
  `LOCALAPPDATA` definido). O site foi ao ar com `npx --yes wrangler@4 deploy` (o `wrangler` não está instalado e o login
  antigo em `~/.config/.wrangler` valeu). Atenção: **nunca rode o binário extraído de um `.deb`** para ver a versão — `--version`
  sobe o daemon e grava estado em `~/.local/share/rustploy/db`.
- Rodados: `make lint`, `make test` por repo, `glacier test` (406), `tests/e2e-updater/run.sh` (11 cenários) e o `npm run
  build` + `check-links` do site (0 quebrados).

## Ainda sem teste real

- `install.ps1`: o atalho do Menu Iniciar (COM do Windows) e a assinatura com o `minisign.exe` (no Windows
  PowerShell 5.1). O resto (consulta, download, SHA-256, assinatura com o `minisign` do Linux, extração, cópia)
  rodou num PowerShell 7 no Linux.
- O `.deb` arm64 instalado numa máquina arm64 (só foi compilado).
- A migração Docker -> Podman com dados reais (só testada com stubs; ver `TODO.md`).
- O `install.sh` numa VPS com systemd de verdade (os testes foram em container, sem systemd): subir o serviço e o
  helper de firewall.
