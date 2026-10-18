# Plano: backend Podman (alternativa ao Docker)

> Status: **implementado e publicado** (daemon 0.1.2, shared 0.4.1; mergeado na `main` em 2026-10-18), validado em Podman rootful, **com pendências de teste** (seção "Pendências", também no `TODO.md`).

## Por quê

`dockerd` + `containerd` ficam residentes o tempo todo (medido nesta máquina:
~100 MB + ~45 MB de RSS). O Podman não tem daemon: o `podman.socket` sobe sob
demanda (socket activation) e cada container paga só um `conmon` (~1–3 MB). O
consumo dos containers em si não muda — a economia é a do motor, ~100–150 MB.

## Como o daemon fala com o motor

1. **bollard** no socket de `[docker].socket_path` (API compatível com Docker).
2. **CLI** em subprocesso: `compose up/down/logs/config`, `pull`, `login/logout`,
   `exec`, `run`, `builder prune`.

O Podman serve (1) sem mudança (`/run/podman/podman.sock`) e tem CLI
equivalente para (2).

## Decisão de desenho

- **Sem campo novo no `config.toml`**: o `rustploy-shared` é publicado no
  crates.io e o parse da config é tudo-ou-nada. O motor é detectado por
  `docker::Engine::detect`: `RUSTPLOY_CONTAINER_ENGINE=podman|docker` ou, sem
  ele, "podman" no `socket_path`.
- **Todo subprocesso passa por `docker::cli()`**: escolhe o binário e exporta
  `DOCKER_HOST=unix://<socket>`. No Podman acrescenta `--remote --url
  unix://<socket>`: o `podman` local **ignora** o `DOCKER_HOST` e operaria no
  storage rootless do usuário do daemon, não no do socket (bug achado no smoke
  test). O `podman compose` delega ao provider `docker-compose` (o plugin
  `docker-compose-plugin` serve; o pacote `docker-compose-v2` do apt conflita
  com ele e não é necessário).
- `builder prune` não existe no Podman → `system prune -f --build`.

## Como usar na VPS (o passo a passo, sem pressupor nada)

### O caminho automático

```
curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh -s -- --engine podman
```

(`--engine podman` combina com `--remote`.) Sem a flag vale `--engine docker`,
o padrão: se o `docker` não existir, o instalador instala o Docker Engine
(`docker-ce`) pelo repositório oficial da Docker, seguindo
https://docs.docker.com/engine/install/ubuntu/ (no Debian usa o repositório
`debian`; em derivados, o codinome do Ubuntu/Debian base; em distro desconhecida,
cai no `docker.io`). Se o `docker` já existe, não mexe nele. A documentação do
site (`instalacao/instalar-daemon`, pt e en) traz os dois caminhos, automático e
manual. **Só funciona com uma release que já tenha o código do
Podman** — as releases anteriores à branch `feat/backend-podman` ignoram a
variável e ainda dependem do `docker.io`.

### O que o instalador faz, e por quê

O rustploy precisa de um "motor de containers" — o programa que de fato cria e
roda os containers. Hoje é o Docker; aqui trocamos pelo Podman. Para o rustploy
conversar com ele são precisas quatro coisas, e o instalador faz as quatro:

1. **Instala o Podman** (`apt install podman`) e confere que a versão é a 4.7
   ou mais nova — antes disso não existe o `podman compose`, que o rustploy usa
   para os serviços Compose. Instala também um "provider" de compose
   (`docker-compose-v2`, `docker-compose-plugin` ou `docker-compose`, o primeiro
   que existir): o `podman compose` é só um repassador, quem executa o
   `docker-compose.yml` é esse programa.
   *Isso vem antes do pacote do rustploy* de propósito: o `.deb` agora aceita
   "docker **ou** podman" como dependência, e com o Podman já instalado o apt
   não puxa o Docker junto (que é justamente a RAM que queremos poupar).

2. **Liga o `podman.socket`**. O Docker fica sempre rodando e escutando num
   arquivo-socket (`/var/run/docker.sock`). O Podman não: o systemd fica de
   guarda no arquivo `/run/podman/podman.sock` e só acorda o Podman quando
   alguém bate nele (o rustploy, no caso); passado um tempo parado, ele dorme de
   novo. É daí que vêm os ~100 MB poupados.

3. **Dá ao usuário `rustploy` acesso a esse socket.** O rustploy roda como o
   usuário `rustploy` (não como root). O socket do Podman nasce
   `root:root` com permissão `0660` — ou seja, só o root entra, e o rustploy
   levaria "permission denied". No teste desta máquina eu resolvi com
   `chgrp antonio` na mão, mas isso **se perde** toda vez que o systemd recria o
   socket (reboot, restart). O instalador grava um arquivo
   `/etc/systemd/system/podman.socket.d/rustploy.conf` com
   `SocketGroup=rustploy`: um "drop-in", um pedaço de configuração que o
   systemd soma ao unit original do Podman *sem mexer nele* e que vale sempre
   que o socket é criado.

4. **Troca o motor no rustploy**, em dois lugares:
   - `/etc/systemd/system/rustployd.service.d/podman.conf` (outro drop-in, sobre
     o `rustployd.service`): acrescenta `After=`/`Wants=podman.socket`,
     para o rustploy só subir depois de o socket do Podman existir (e pedir que
     ele exista). O unit do pacote só *ordena* depois do `docker.service`
     (`After=`), sem `Wants=`: o rustploy nunca puxa o Docker. (Tentei antes
     zerar o `Wants=docker.service` do unit original com um `Wants=` vazio no
     drop-in; **não funciona**, o systemd mantém a dependência — por isso o
     unit do pacote foi mudado.) Também define `RUSTPLOY_CONTAINER_ENGINE=podman`.
   - `/etc/rustploy/config.toml`: a linha `socket_path` da seção `[docker]`
     passa a ser `"/run/podman/podman.sock"`. (A chave continua se chamando
     `[docker]` por compatibilidade; o rustploy identifica o Podman por esse
     caminho ou pela variável acima.)

Por fim reinicia o rustploy. Por que *drop-ins* e não editar o
`rustployd.service` do pacote? Porque o arquivo do pacote é sobrescrito a cada
atualização, e os drop-ins ficam em `/etc/` e sobrevivem a ela.

### O que o instalador NÃO faz (e você precisa saber)

- **Numa instalação nova não há o que migrar.** Já numa VPS que roda em Docker, a
  atualização **pergunta** o motor e, se escolher Podman, MIGRA (ver "Migração
  Docker -> Podman" abaixo). Só containers `rp_*` são tratados.
- **Não desliga o Docker.** Para de fato liberar a memória:
  `sudo systemctl disable --now docker docker.socket` — só depois de migrar tudo.
- Só **rootful** (Podman rodando como root, o socket em `/run/podman/`). Rootless
  não é suportado: o ingress alcança os containers pelo IP deles e usa as portas
  80/443.

### Fazendo à mão (sem o instalador)

```
sudo apt install podman docker-compose-v2     # ou docker-compose-plugin / docker-compose
sudo mkdir -p /etc/systemd/system/podman.socket.d /etc/systemd/system/rustployd.service.d
printf '[Socket]\nSocketGroup=rustploy\nSocketMode=0660\n' | sudo tee /etc/systemd/system/podman.socket.d/rustploy.conf
printf '[Unit]\nAfter=podman.socket\nWants=podman.socket\n[Service]\nEnvironment=RUSTPLOY_CONTAINER_ENGINE=podman\n' | sudo tee /etc/systemd/system/rustployd.service.d/podman.conf
sudo sed -i 's|^socket_path *=.*|socket_path = "/run/podman/podman.sock"|' /etc/rustploy/config.toml
sudo systemctl daemon-reload && sudo systemctl enable --now podman.socket && sudo systemctl restart podman.socket rustployd
```

## Migração Docker -> Podman na atualização (2026-10-18)

`install.sh` (e o `make install-rustploy`, que chama `install.sh engine`) pergunta o
motor quando há terminal; sem terminal mantém o atual. A escolha "Docker" fica em
`/etc/rustploy/engine`. Escolhendo Podman numa VPS em Docker:

Reaproveita TUDO do Docker (as 9 etapas estão em `instalacao/instalar-daemon` nas docs):
`install_podman`; serviços no ar (API, precisa de `python3`); backup (sem o registry);
espaço; **imagens** (`docker save | podman load` + `podman tag` para o nome completo,
ANTES de parar nada); parar `rustployd` e `rp_*` (+ `--restart=no`); **todos os volumes**
(`cp -a`; opções NFS/bind são recriadas sem copiar; anônimo órfão fica de fora); **redes**
de usuário/compose (bridge, sem sub-rede fixa); `configure_podman`; reinicia; redeploy.
Não reaproveitado: cache de build, containers (o deploy recria), containers não-`rp_*`.

Falha antes da troca do motor = `rollback_docker` (religa o que parou, rustployd volta
no Docker). O Docker nunca é desligado nem apagado. Testado de ponta a ponta com
`docker`/`podman`/`systemctl` falsos e uma API falsa num Ubuntu 24.04: caminho feliz,
rollback, recusa sem `--yes`, `--dry-run` sem alterações, pergunta interativa (pty) e a
escolha lembrada. **Não testado com um Docker de verdade e dados de verdade** (ver
"Pendências").

## Lacunas achadas ao olhar a infra real do usuário (2026-10-18)

O export da VPS (`rustploy-infra.zip`: 20 serviços, 13 Git + 7 Compose, 5 `host_port`) mostrou
o que o smoke test não cobria. Corrigido no `configure_podman`:

- **`restart:` no Compose (6 de 7 stacks)**: no Docker quem religa no boot é o `dockerd`; no
  Podman é o `podman-restart.service`, que vem **desligado**. Sem ele, reboot = bancos fora.
- **Nome curto na CLI** (`podman pull postgres:17`): falha, o Podman da distro não tem
  `unqualified-search-registries`. O caminho do daemon (API compat) já normalizava. Arquivo
  `registries.conf.d/rustploy.conf`.
- **`pids_limit`**: Podman 2048 por container, Docker ilimitado. `containers.conf.d/rustploy.conf`.
- **`/etc/docker/daemon.json`** (logs, mirrors, DNS) não migra: o instalador avisa.

Testado no Podman rootful real: Compose com `healthcheck` + `depends_on: service_healthy`,
`ports` publicadas, `configs` com `content:` e `restart:` -> Live (o app esperou o banco);
serviço Application com limite de memória/CPU e imagem de nome curto -> Live; `exec -i`,
`run --rm --network -e VAR`, `compose config --format json` e `system prune --build` pela CLI remota.

Ainda sem teste (a VPS tem e eu não consegui reproduzir): interação com **ufw** (portas
publicadas por Compose, como o Supabase 8000, e FORWARD), saída para a internet de dentro dos
containers no netavark, conflito do **aardvark-dns** se o host já usa a porta 53, capacidades
padrão (o Podman não dá NET_RAW), e o **reboot** com `restart:` de verdade.

## Resultado do smoke test (Podman 5.7.0 rootful, netavark, daemon real)

Passaram: deploy de imagem de registry até `Live`; redeploy (rename + alias de
rede, container velho em `Pruning`); rota do ingress aponta para o IP do
container na bridge `podman1` (10.89.0.x) e o backend responde 200; troca de IP
no redeploy acompanhada pela rota; stack Compose de 2 serviços `Live`; DNS por
nome entre serviços; label `com.docker.compose.project`; build de imagem pela
API (`/build`, Buildah). Rootless: tudo igual, mas o teardown de compose avisa
`rootless netns: kill network process: permission denied` (não afeta o rootful).
Não coberto: build de Git/Archive de ponta a ponta, registry embutido
(`login` no `podman`), `logs` de compose pela GUI, jobs one-shot.

## Riscos (antes do smoke test)

- DNS entre containers: o Podman usa `netavark` + `aardvark-dns`; o alias
  `rp_<safe_name>` e `attach_network_alias` (disconnect+connect) precisam
  resolver.
- IPs de container na rede do projeto (`live_replica_ips`) — o ingress faz proxy
  direto para eles.
- Labels `com.docker.compose.project` (índice de stacks) — o provider
  `docker-compose` os grava; o `podman-compose` não.
- Build: `images::build` manda um tar à API de build; no Podman isso é o
  Buildah (sem BuildKit — Dockerfiles com `--mount=type=cache` funcionam,
  `# syntax=` custom não).
- Unit de systemd: `After=docker.service` → `podman.socket` (hoje não alterado:
  ajuste manual na VPS).

## Pendências (o usuário vai testar depois, 2026-10-18)

O que **não** foi coberto no smoke test com o Podman rootful e fica por testar:

- Upload de **zip (Archive)**: o build usa o mesmo caminho do Git (que passou),
  mas o upload em si não foi exercitado.
- **TLS/ACME com domínio real** (o domínio de teste só provou o redirect 301 e a
  falta de certificado; o encaminhamento ao container foi provado pela rota de
  porta TCP).
- **Volumes** (bind e nomeados) e a persistência de dados de **bancos**; os
  volumes do Docker antigo foram apagados na migração.
- **Jobs one-shot (Schedules)** e o **registry embutido** (`login` pelo `podman`).
- **A migração Docker -> Podman com dados reais** (`install.sh` e `make install-rustploy`):
  só foi exercitada com stubs. Comece com `--dry-run` e confira o backup em `/var/backups/`.
- Subir os serviços reais do usuário (todos `Stopped`) e conferir que tudo volta.
- `install.sh --engine docker` e `--engine podman` de ponta a ponta numa VPS
  limpa (o `install_docker` e o `install_podman` foram testados isolados num
  Ubuntu 24.04 limpo, mas o script inteiro, com download da release, não).

Defeitos do rustploy (não do Podman) vistos de passagem, sem correção:

- ~~`ServiceDelete` não para nem remove os containers do serviço~~ (corrigido: ele
  reaproveita o `ServiceStop` e remove o que sobrar; Compose usa `compose down`, que
  preserva os volumes nomeados).
- Não há comando para remover container em execução (sem `force`) nem para
  parar um container solto pela API.

## Fora de escopo

O `postinst` do `.deb` e o `rustployd.service` do pacote continuam como eram
(Docker por padrão; o Podman entra por drop-in, só quando se pede). A tela
"Docker" da GUI/webui mantém o nome. O `.deb` agora depende de
`docker.io | docker-ce | podman`.
