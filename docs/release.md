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
  certo (`SecretGenerate`/`SecretRotate`) está na Fase G do `docs/plano-credenciais-cifradas-e-storage.md`. O manual e as docs
  **não** recomendam mais aquela receita. A `v0.1.36` tem o texto antigo: **não assine o rascunho**; use a v0.1.37.

## v0.1.35 (2026-11-03): credenciais cifradas, renomear e storage — daemon por `path`

- **O daemon referencia o `rustploy-shared` por `path`** (`rustploy-daemon/crates/daemon/Cargo.toml`, `version` + `path`),
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
