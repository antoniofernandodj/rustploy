# Rustploy

PaaS single-node em Rust (alternativa ao Dokploy). Este repositório é o
**agregador**: três repositórios reunidos por git submodules.

| Submodule | O quê |
|---|---|
| [`rustploy-daemon`](https://github.com/antoniofernandodj/rustploy-daemon) | daemon (`rustployd`), importer, webui — **leia o README dele** |
| [`rustploy-gui`](https://github.com/antoniofernandodj/rustploy-gui) | cliente desktop (glacier-ui) |
| [`rustploy-shared`](https://github.com/antoniofernandodj/rustploy-shared) | tipos, protocolo e catálogo de templates ([crates.io](https://crates.io/crates/rustploy-shared)) |

## Instalar

Servidor (Debian ou Ubuntu, como root) e app desktop, cada um com um comando (baixam a última release):

```bash
curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh              # daemon na VPS
curl -fsSL https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.sh | sudo sh -s -- gui    # app desktop (Linux)
```

```powershell
irm https://raw.githubusercontent.com/antoniofernandodj/rustploy/main/install.ps1 | iex                        # app desktop (Windows)
```

Opções, requisitos e o passo a passo: [Instalação rápida](https://rustploy-docs.antoniofernandodj.workers.dev/instalacao/instalacao-rapida/).

## Desenvolver

```bash
git clone --recurse-submodules https://github.com/antoniofernandodj/rustploy.git
# ou, se já clonou sem a flag:
git submodule update --init --recursive
```

`docs/` guarda os planos, relatórios e o índice de código (`docs/indice/`).
Cada submodule compila sozinho; o `Makefile` do `rustploy-daemon` roda
build/test/fmt/clippy nos três (`make -C rustploy-daemon check`).
