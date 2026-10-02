# CLAUDE.md

**A referência deste projeto é o [`rustploy-daemon/AGENTS.md`](rustploy-daemon/AGENTS.md).** Leia-o.

Este repo é só o **agregador**: reúne por git submodules os três repositórios do
projeto e guarda o que é transversal a eles.

```
rustploy/                     ← este repo
├── docs/                     planos, relatórios e docs/indice/ (índice gerado)
├── .github/workflows/        release.yml (checkout recursivo, builda os três)
├── rustploy-daemon/          daemon, importer, indexer, webui, Makefile, AGENTS.md
├── rustploy-gui/             cliente glacier-ui
└── rustploy-shared/          crate publicado no crates.io
```

O que procurar:

| Assunto | Onde |
|---|---|
| **Achar código gastando pouco token: `docs/indice/INDEX.md` antes de qualquer grep/leitura** | aqui, `docs/indice/` (regere com `make index` em `rustploy-daemon/`) |
| Tudo o mais: API de agente, convenções, build/teste, arquitetura, história | `rustploy-daemon/AGENTS.md` |
| Os três repos, como publicar o `rustploy-shared`, regra do `glacier-ui` | `rustploy-daemon/AGENTS.md`, Parte 2 — "Os três repos" e Convenções |

Clone com `git clone --recurse-submodules`; sem a flag os submodules ficam vazios.
Mudou código num submodule? Commit e push **lá**, e depois commite aqui o novo
ponteiro (`git add rustploy-daemon …`). Planos e relatórios por assunto ficam em
`docs/`; o cabeçalho de cada um diz se já foi implementado.
