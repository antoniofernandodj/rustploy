# CLAUDE.md

**A referência deste projeto é o [`AGENTS.md`](AGENTS.md).** Leia-o.

Este arquivo existia em paralelo com o `AGENTS.md`, os dois descrevendo a
arquitetura por conta própria — o que garantia que um dos dois estaria
desatualizado, e por um bom tempo estiveram os dois, cada um de um jeito. Em
2026-08-28 o conteúdo foi fundido no `AGENTS.md` e este virou um ponteiro.
Não volte a documentar arquitetura aqui.

O que procurar lá:

| Assunto | Onde |
|---|---|
| Operar um rustploy por HTTP (a API de agente) | Parte 1 — Manual de Controle por Agente |
| Regra do `glacier-ui` (nunca `path`/`[patch]`, sempre publicar) | Parte 2 — Convenções |
| GUI e webui são **dois** clientes; feature de UI entra nos dois | Parte 2 — Convenções |
| Ferramental e convenções de Luau, armadilhas de `.gv`/GSS | Parte 2 — Convenções |
| Comandos de build e de teste (os pacotes **não** se chamam `daemon`/`shared`) | Parte 2 — Build & Run |
| Config (o parse é tudo-ou-nada) | Parte 2 — Configuração |
| Crates, protocolo, internos do daemon e da GUI | Parte 3 — Arquitetura |
| Máquina de estados do deploy e onde a causa de uma falha aparece | Parte 3 — Arquitetura |
| Decisões já revertidas (SurrealDB, UDS, TUI…) e buracos conhecidos | Parte 4 — História |

Planos e relatórios por assunto ficam em `docs/`; o cabeçalho de cada um diz se
já foi implementado.
