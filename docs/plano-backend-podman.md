# Plano: backend Podman (alternativa ao Docker)

> Status: **implementado e validado em rootful** (branch `feat/backend-podman`, 2026-10-17). Falta só decidir o instalador/unit (ver "Fora de escopo").

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

(`--engine podman` combina com `--remote`.) Sem a flag, nada muda: o Docker
continua sendo o padrão. **Só funciona com uma release que já tenha o código do
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
     o `rustployd.service`): o unit original diz "inicie depois do
     `docker.service` e peça para ele subir" (`After=`/`Wants=`). O drop-in zera
     essas duas linhas e põe `podman.socket` no lugar — assim o rustploy não
     acorda o Docker à toa, e só sobe depois de o socket do Podman existir.
     Também define `RUSTPLOY_CONTAINER_ENGINE=podman`.
   - `/etc/rustploy/config.toml`: a linha `socket_path` da seção `[docker]`
     passa a ser `"/run/podman/podman.sock"`. (A chave continua se chamando
     `[docker]` por compatibilidade; o rustploy identifica o Podman por esse
     caminho ou pela variável acima.)

Por fim reinicia o rustploy. Por que *drop-ins* e não editar o
`rustployd.service` do pacote? Porque o arquivo do pacote é sobrescrito a cada
atualização, e os drop-ins ficam em `/etc/` e sobrevivem a ela.

### O que o instalador NÃO faz (e você precisa saber)

- **Não move o que já roda no Docker.** Containers, imagens e volumes do Docker
  continuam lá; o Podman começa vazio. Depois de instalar, refaça o deploy de
  cada serviço (bancos precisam de atenção: os dados estão em volumes do Docker).
  O instalador avisa se achar um Docker na máquina.
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
printf '[Unit]\nAfter=\nWants=\nAfter=network.target podman.socket\nWants=podman.socket\n[Service]\nEnvironment=RUSTPLOY_CONTAINER_ENGINE=podman\n' | sudo tee /etc/systemd/system/rustployd.service.d/podman.conf
sudo sed -i 's|^socket_path *=.*|socket_path = "/run/podman/podman.sock"|' /etc/rustploy/config.toml
sudo systemctl daemon-reload && sudo systemctl enable --now podman.socket && sudo systemctl restart podman.socket rustployd
```

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

## Fora de escopo

O `postinst` do `.deb` e o `rustployd.service` do pacote continuam como eram
(Docker por padrão; o Podman entra por drop-in, só quando se pede). A tela
"Docker" da GUI/webui mantém o nome. O `.deb` agora depende de
`docker.io | docker-ce | podman`.
