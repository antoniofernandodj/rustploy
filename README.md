# Rustploy

PaaS single-node em Rust (alternativa ao Dokploy). Este repositório é o
**agregador**: três repositórios reunidos por git submodules.

| Submodule | O quê |
|---|---|
| [`rustploy-daemon`](https://github.com/antoniofernandodj/rustploy-daemon) | daemon (`rustployd`), importer, webui — **leia o README dele** |
| [`rustploy-gui`](https://github.com/antoniofernandodj/rustploy-gui) | cliente desktop (glacier-ui) |
| [`rustploy-shared`](https://github.com/antoniofernandodj/rustploy-shared) | tipos, protocolo e catálogo de templates ([crates.io](https://crates.io/crates/rustploy-shared)) |

```bash
git clone --recurse-submodules https://github.com/antoniofernandodj/rustploy.git
# ou, se já clonou sem a flag:
git submodule update --init --recursive
```

`docs/` guarda os planos, relatórios e o índice de código (`docs/indice/`).
Cada submodule compila sozinho; o `Makefile` do `rustploy-daemon` roda
build/test/fmt/clippy nos três (`make -C rustploy-daemon check`).
