# Plano: backend Podman (alternativa ao Docker)

> Status: **em implementação** (branch `feat/backend-podman`, 2026-10-17).

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
  `DOCKER_HOST=unix://<socket>` (o `podman compose` delega ao provider
  `docker-compose`, que lê essa variável).
- `builder prune` não existe no Podman → `system prune -f --build`.

## Como usar (host)

```
sudo apt install podman docker-compose-v2 uidmap
sudo systemctl enable --now podman.socket     # /run/podman/podman.sock (rootful)
# config.toml:  [docker] socket_path = "/run/podman/podman.sock"
# opcional:     RUSTPLOY_CONTAINER_ENGINE=podman
```

Rootful é o caminho suportado (o ingress/registry usam portas baixas e o IP do
container é roteado pelo host). Rootless fica fora de escopo.

## Riscos a validar (smoke test)

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

Instalador (`postinst`, `.service`) e `docker.js`/tela "Docker" da GUI
continuam com o nome Docker.
