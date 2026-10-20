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

Os instaladores (`install.sh`, `install.ps1`) leem a **última** release pela API do GitHub e
procuram os arquivos pelo padrão do nome, então nada precisa ser editado a cada release.

## Passo a passo

1. Tudo commitado e enviado (submódulos primeiro, depois o agregador com os ponteiros).
2. **Teste sem publicar:** `gh workflow run release.yml --ref main`. O job `release` só roda em tag,
   então isso só compila e guarda os pacotes como artefatos (`gh run download <id>`).
3. Confira os artefatos (`dpkg-deb -I` mostra `Depends`, inclusive a versão mínima da `libc6`).
4. `git tag -a vX.Y.Z -m "..." && git push origin vX.Y.Z` publica a release.
5. Teste o comando de uma linha de verdade num Debian limpo (`docker run debian:12`).

Uma tag cuja release **não** foi publicada pode ser apagada e refeita no commit corrigido
(`git tag -d`, `git push origin :refs/tags/vX.Y.Z`). Depois de publicada, não mexa nela: crie outra.

## Por que os .deb compilam num Ubuntu 22.04

A versão mínima da glibc de um binário vem de **onde ele é compilado**. Um build no runner novo
(24.04, glibc 2.39) exigiria `libc6 >= 2.39` e não instalaria no Debian 12. Dentro de um container
`ubuntu:22.04` o daemon exige `libc6 >= 2.34` e o app `>= 2.35`. Isso aparece na tabela de
compatibilidade em `rustploy-docs` (`instalacao/requisitos`). Mudou o container? Mudou a tabela.

## Armadilhas que já apareceram

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
  templates_render` no `gui` à parte.
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

## Deploy automático nos servidores (job `deploy-servers`)

Depois do job `release` (só em tag), o `release.yml` entra por SSH nos dois servidores do dono
(`itemize@179.198.122.159` e `antonio@chiquitos.tech`) e roda `cd ~/rustploy && git pull && git submodule
update --init --recursive && make install-rustploy`. Precisa de dois segredos em *Settings > Secrets and
variables > Actions*; sem eles o job é **pulado** (a release sai normalmente):

- `DEPLOY_SSH_KEY`: chave privada sem senha, com a pública no `~/.ssh/authorized_keys` dos dois.
- `DEPLOY_KNOWN_HOSTS`: `ssh-keyscan -t ed25519 179.198.122.159 chiquitos.tech`.

Os servidores precisam de `sudo` sem senha para esse usuário (o `make install-rustploy` usa `sudo`
dpkg/systemctl) e compilam o daemon na própria máquina, então o job demora (teto de 90 min). Um servidor
fora do ar não impede o outro (`fail-fast: false`).

## Ainda sem teste real

- `install.ps1`: o atalho do Menu Iniciar (COM do Windows). O resto (consulta, download, SHA-256, extração, cópia)
  rodou num PowerShell no Linux.
- O `.deb` arm64 instalado numa máquina arm64 (só foi compilado).
- A migração Docker -> Podman com dados reais (só testada com stubs; ver `TODO.md`).
- O `install.sh` numa VPS com systemd de verdade (os testes foram em container, sem systemd): subir o serviço e o
  helper de firewall.
