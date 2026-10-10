# Plano: nomes descritivos em toda a base (GUI glacier + webui)

Estado: **implementado em 2026-10-10** (chaves de contexto e handlers, GUI e webui). Decisões da seção 6: chaves **e** handlers, tudo em inglês, `manifest_`, palpites de `mdb_`/`pdc_`/`ss_` confirmados, nomes longos aceitos. Variáveis locais ficaram para a segunda rodada.
Data: 2026-10-10.

## 1. O problema, em uma frase

Variáveis como `ctx.dc_kinds` obrigam quem lê a **adivinhar** uma sigla (`dc` =
docker cleanup). O mesmo vale para `njob_`, `ns_`, `f_`, `gp_`, `hc_`, `imp_`,
`iac_`, `ntok_`, `penv_`… A pessoa nova (ou você daqui a seis meses) abre
`ctx.f_hc_retries` e tem que cruzar três arquivos para descobrir que é "tentativas
do healthcheck, no formulário do serviço".

A regra que queremos: **o nome se explica sozinho, sem glossário.**

## 2. Tamanho real do trabalho (medido, não estimado)

Contagem de ocorrências de identificadores cujo prefixo é uma sigla curta:

| Onde | Famílias de sigla | Identificadores distintos | Ocorrências |
|---|---|---|---|
| GUI (`.luau`, `.gvb`, testes `.rs`) | ~25 | ~600 | ~2.500 |
| webui (`.js`, `index.html`, testes `.rs`) | ~15 | ~250 | ~700 |

Maiores famílias na GUI: `njob_` (41 nomes, 270 usos), `svc_` (60, 244), `ns_` (38,
205), `f_` (36, 203), `busy_` (121, 165 — derivado dos nomes de handler), `imp_`
(30, 136), `dc_` (25, 126), `eng_` (21, 106), `gp_` (16, 91), `ex_` (20, 73).
Na webui as mesmas famílias existem em camelCase (`njobKind`, `dcKind`, `fHcRetries`…).

A webui e a GUI **não compartilham código**, só convenção de nome (a webui é "tradução
literal" da GUI). Hoje o mesmo conceito tem dois nomes (`dc_kinds` / `dcKinds`).
Renomear os dois lados com **o mesmo vocabulário** mantém essa correspondência.

## 3. O glossário (a decisão central)

Cada sigla vira a palavra inteira. Confirmadas pelo código (não chutadas):

| Sigla | Significa | Prefixo novo |
|---|---|---|
| `dc_` | docker cleanup (limpeza automática do Docker; `DockerCleanupConfig`) | `docker_cleanup_` |
| `eng_` | deploy engine (a tela da fila de deploys) | `deploy_engine_` |
| `njob_` | new job (janela "Novo job") | `new_job_` |
| `ns_` | new service (wizard "Novo serviço") | `new_service_` |
| `np_` | new project (janela "Novo projeto") | `new_project_` |
| `ntok_` | new (registry) token | `new_registry_token_` |
| `f_` | form do serviço (aba General e afins) | `service_form_` |
| `gp_` | git provider (cadastro de GitHub/Gitea) | `git_provider_` |
| `hc_` | healthcheck | `healthcheck_` |
| `imp_` | import (importar bundle/serviço) | `import_` |
| `ex_` | export (exportar serviço) | `export_` |
| `iac_` | infrastructure as code (export/import de manifesto) | `manifest_` (ver 6.3) |
| `penv_` | project env (variáveis do projeto) | `project_env_` |
| `sec_` | secret (do projeto) | `project_secret_` |
| `lw_` | log window | `log_window_` |
| `mig_` | migração de banco | `migration_` |
| `mdb_` | banco do servidor compartilhado (managed/shared database) | `shared_database_` |
| `pdc_` | pre-deploy check (gate de pré-deploy) | `pre_deploy_check_` |
| `ss_` | server settings (aba Settings → Web) | `server_settings_` |
| `sys_` | system (CPU/mem/disco do host) | `host_` |
| `proj_` / `svc_` / `dep_` / `prov_` | project / service / deployment / provider | `project_` / `service_` / `deployment_` / `provider_` |
| `erro_` | mistura português/inglês | `error_` |

Três siglas **não** consegui confirmar só pelo nome e preciso da sua palavra:
`mdb_`, `pdc_` e `ss_` (a coluna acima é o melhor palpite a partir do uso).

## 4. Regras de nome (o que "descritivo" quer dizer aqui)

1. Sem sigla de 2–4 letras. Palavra inteira ou composta (`service_form_healthcheck_retries`).
2. **Um idioma só: inglês** (hoje há `erro_`, `ntok_apontar`, `id_curto` misturados com
   inglês). Textos exibidos ao usuário continuam em português; só os identificadores mudam.
3. O nome diz **o quê**, não **onde**: `docker_cleanup_enabled`, não `dc_enabled`.
4. Contagem = `*_count`, texto pronto para exibir = `*_label`, flag booleana =
   `is_*`/`has_*`/`can_*`. (Já há bastante disso; falta uniformizar `_msg`/`_err`/`_text`.)
5. O prefixo da tela **fica sempre**, mesmo dentro do arquivo dela: as chaves de `ctx`
   são globais da janela (um espaço de nomes plano), então `new_job_name` não pode
   encurtar para `name`. É o custo do contexto plano.

## 5. Como fazer sem quebrar (e o que pode quebrar)

Abordagem: **um mapa de renomeação gerado a partir do glossário**, aplicado por script
com fronteira de palavra, e conferido por três redes de segurança. Nada de
substituição manual arquivo a arquivo.

Etapas:

1. **Gerar o mapa** (`nome_antigo → nome_novo`) a partir de todos os identificadores com
   sigla. Você revisa o mapa **antes** de qualquer arquivo mudar (é uma tabela de ~850 linhas).
2. **GUI**: aplicar em `views/scripts/**/*.luau`, `views/**/*.gvb` e `tests/*.rs`.
3. **webui**: o mesmo vocabulário em camelCase (`dcKinds` → `dockerCleanupKinds`), em
   `*.js`, `index.html` e nos testes headless de `api/web_ui.rs`.
4. Rodar as redes de segurança abaixo, e só então commitar (um commit por repo).

Redes de segurança que já existem: `cargo test` do `rustploy-gui` (renderiza todas as
telas), os 7 testes headless da webui (Chrome de verdade) e o novo teste de combinadores.
A que **falta** e eu criaria antes: um teste que falha se algum `@nome` de `.gvb` não for
escrito por nenhum script (chave inexistente hoje aparece como texto vazio, em silêncio).

Onde pode quebrar — e o que faço em cada caso:

| Risco | Por quê | Tratamento |
|---|---|---|
| **Chaves montadas em tempo de execução** | `"busy_" .. ação`, `"erro_" .. campo`, `ctx[k]` em laço (19 usos) | Revisão manual: cada construção dinâmica é listada e ajustada à mão |
| **`erro_<campo>` é derivado do `formControl`** | o motor prefixa `erro_` ao nome do campo | Renomear o campo muda o `erro_…` junto; conferir par a par |
| **`busy_<handler>`** | a chave de "ocupado" nasce do nome do handler | Renomear handler e chave juntos (`busy_actions.luau` como fonte única) |
| **Campos que vêm do daemon** | JSON/Rust compartilhado não pode mudar de nome | O mapa só toca identificadores **locais** da GUI/webui; nomes de campos do `rustploy-shared` ficam intocados (verifico um a um contra o `shared`) |
| **Nome já em uso** | `proj_` → `project_` pode colidir com `project_…` existente | O gerador do mapa recusa colisão e para |
| **Documentação** | `AGENTS.md`, índice (`docs/indice`), planos citam nomes | `make index` + busca nos `.md`; planos históricos ganham só a nota "renomeado em 2026-10" |

## 6. Decisões que preciso de você

1. **Escopo**: só as **chaves de contexto** (o `ctx.…` e os `@…` dos `.gvb`, que é o que
   você apontou), ou também os **nomes de handler** (`dc_save`, `njob_create`…) e as
   variáveis **locais** dos `.luau`? Os handlers compartilham as mesmas siglas e viram
   `busy_<handler>`; deixá-los de fora deixa a base pela metade. Minha recomendação:
   chaves **e** handlers agora; variáveis locais (`k`, `v`, `ok`, `r`) numa segunda rodada,
   porque aí o ganho é menor e o risco de regressão maior.
2. **Idioma**: confirmar que os identificadores ficam **todos em inglês**.
3. **`iac_`**: `manifest_` (o que a tela faz: exportar/importar o manifesto) ou
   `infrastructure_as_code_` (fiel à sigla, mas longo)?
4. As três siglas que não confirmei: **`mdb_`**, **`pdc_`**, **`ss_`** — qual a palavra certa?
5. **Comprimento**: nomes como `service_form_healthcheck_retries` (32 letras) são o preço
   de não ter sigla. Aceita? (Alternativa: `healthcheck_retries_input` — mais curto, mas
   perde a informação de que é do formulário de serviço.)

## 7. O que este plano **não** faz

- Não muda comportamento, layout nem estilo (é renomeação pura).
- Não toca em Rust fora dos testes: o daemon e o `rustploy-shared` ficam como estão.
- Não traduz os textos mostrados ao usuário.
