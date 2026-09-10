# Plano: reforma do `rustploy-gui` sobre o glacier-ui moderno (0.87 → 0.102)

Estado: **em andamento** — Ondas 0, 1 (parcial) e 2 feitas; spinner trocado.
Data: 2026-09-10.

## O que já foi (branch `reforma-gui-glacier-0.102`)

| | commit | resumo |
|---|---|---|
| **Onda 0** | `chore(rustploy-gui): bump glacier-ui 0.87 → 0.102.1` | dependência subida; `cargo check --workspace`, `test` (13+45), `clippy`, `fmt` verdes; nenhuma quebra do intervalo toca o rustploy |
| **Spinner** | (junto da Onda 2) | `LoadingRow` troca o glifo `⟳` estático pelo `<spinner>` do motor (anel girando) |
| **Onda 2** | `refactor(rustploy-gui): sidebar vira <drawer>` | `shell.gv` usa `<drawer value="menu">`; `.sidebar` no `.gss` fica só com a cor; somem os `@media` de sidebar (shell + nav_item); `☰` na topbar via `app:drawer::toggle:menu`; `init()` semeia `menu="true"`. Teste reescrito. |
| **Onda 1** (parcial) | `refactor…: "Novo projeto"…` + `refactor…: edit_project/penv_add/secret_add…` | 4 formulários no `<form>` validado (`rules=`/`msg=`/`on_validation_error`/`type="submit"`); ~7 escadas `if campo=="" …` removidas dos handlers; `:invalid` no `app.gss`; teste da fiação de validação na árvore avaliada. |

**Falta da Onda 1:** `login.gv` (o `<ComboEdit>` de URL — o `connect()` já faz um
`normalize_url` que é validação real, não só "não-vazio"; baixa prioridade),
`service.gv` (form `general` multi-provider — o `f_gen_port` ganharia
`gte:1|lte:65535`, mas a tela é grande e casa melhor com a Onda 3/4) e
`new_service.gv`/`new_job_window.gv` (validação **condicional** entre campos e
passo — pertence à Onda 5, com `<Wizard>`).

**Faltam as Ondas 3–7** (tabelas, abas, wizard, chrome, autocomplete) — abaixo,
inalteradas.

### Polish em cima da reforma (não é onda, mas anda junto)

| commit | resumo |
|---|---|
| `fix…: spinner nos estados "no ar" + alinha campo do <spinbox>` | (1) `.spin_field { padding: 6 12 }` casa a altura do campo com os degraus ▴▾ nos 8 `<spinbox>` (healthcheck/réplicas/dc_hours/njob_hours). (2) flag `*_busy` em cada fluxo com RPC no ar → `<row class="busy_row"><spinner if="{*_busy}"/><text>{*_msg}</text></row>`: novo serviço/projeto/job/token e o picker Git (conta→repo→branch) de serviço e de job. O "carregando branches…" do serviço, que ficava preso, agora atualiza ao terminar. |
| `feat…: Infra as Code importa/exporta um .zip (arquivo), validado` | os 4 `<textarea>` (2 export readonly + 2 import) saem. Exportar → `save_file` grava um `.zip` com `rustploy.yml` + `rustploy.vars.toml`; Importar → `open_file` escolhe o `.zip` e `manifest_zip_read` valida (exatamente um .yml e um .toml na raiz, nada mais). O motor tem `zip_dir` mas não o inverso → `manifest_zip_read`/`manifest_zip_write` entram como **extensão da camada Luau** (`src/manifest_zip.rs` + `.lua_extension(...)` no builder), com o mesmo crate `zip` do glacier-ui (cargo deduplica). 4 testes unitários. Um `confirm{}` no import mostra prune/deploy antes de aplicar; os dois checkboxes agora são semeados em `init` e passados como parâmetro (não relidos do ctx depois dos diálogos). |
| `fix…: bump glacier-ui 0.103.0` | **glacier-ui 0.103.0** (publicado): `render_current` embrulha a tela num `Stack` fixo, então abrir menu/diálogo/toast não zera mais o deslocamento dos `<scrollable>` / o foco / a seleção — era o pulo pro topo no clique-direito num `<input>` dentro de área rolável. Com o conserto no motor, o workaround `set_input_context_menu(false)` foi revertido (o menu do botão direito volta, funcionando, inclusive nas janelas-filhas). |

---

Escopo: `crates/rustploy-gui/views/**` (`.gv` + `.gss` + Luau) e, por reflexo
obrigatório (AGENTS.md, "Toda feature de UI vive em dois lugares"),
`crates/daemon/webui/**`. Sem mudança no `glacier-ui` — tudo abaixo já existe
publicado na 0.102.1.

Relacionado: `docs/plano-widgets-glacier-0.68.md` (a rodada anterior, mesma
ideia, feita em 2026-09-01) e `docs/plano-convergencia-templates-gui-webui.md`
(a divergência GUI↔webui, que várias ondas abaixo fecham de graça).

---

## 1. Ponto de partida

O `rustploy-gui` está preso na **glacier-ui 0.87** (`crates/rustploy-gui/Cargo.toml`).
O motor está na **0.102.1**. No intervalo, o glacier ganhou exatamente as peças
que o rustploy hoje escreve à mão:

| Versão | O que entrou | O que isso substitui no rustploy |
|---|---|---|
| **0.92** | `<drawer>`, `<tableview>`/`<tableheader>`, `<columnview>`, `<autocomplete>` | a sidebar com `@media` manual; ~15 tabelas montadas com `thead`+`for-each`+`<hr>`; os `<select>` de repo/branch |
| **0.94** | `<wizard>` / `<wizardnav>` | os passos manuais `<if cond="{ns_step}" equals=…>` do "Novo serviço" e do "Novo job" |
| **0.96** | série múltipla nos gráficos (`series=`), `<scatter>`/`<areachart>` | (não usado hoje — abre espaço para Monitoring com histórico) |
| **0.97** | atributos que aceitam **chave OU JSON literal** (`columns`, `colors`, `bands`, `items` de menu) | `ctx.*_json` publicados só para alimentar tabela/menu |
| **0.101** | preset `minimo` já nasce com `<drawer value="menu">` + `☰` | a prova de que a sidebar-gaveta é o caminho oficial |
| **0.102** | **validação declarada no `<form>`** (`rules=`, `msg=`, `on_validation_error`, `validate_on`, `:invalid`, `<button type="submit">`) | **todo** `if campo == "" then ctx.erro_X = "…"; return end` espalhado por 8 handlers |

Inventário atual do cliente (`views/`):

| | linhas |
|---|---|
| `.gv` (13 telas/janelas + 10 componentes) | ~5.290 |
| `.gss` (`app.gss` + `<style>` embutido por template) | ~232 + inline |
| Luau (`scripts/**`) | ~6.775 |

As duas telas gigantes — `home.gv` (1.381) e `service.gv` (1.159) — são quase
inteiramente tabela e aba montadas à mão. `shell.gv` (673) é sidebar +
roteamento + editores kv. É aí que a reforma paga.

---

## 2. Onde o rustploy escreve à mão hoje o que o motor já faz

### 2.A — Formulários: validação é `if` no handler

O padrão, repetido: cada campo tem `form_control="X"` + `on_change="field:X"`, e
o handler do submit começa com uma escada de checagens que grava `ctx.erro_X` e
dá `return`. Ocorrências mapeadas:

| Arquivo / handler | campo | checagem manual hoje |
|---|---|---|
| `handlers/projects.luau:106` | `new_proj_name` | vazio → "informe um nome" |
| `handlers/projects.luau:192` | `edit_proj_name` | vazio → "informe um nome" |
| `handlers/projects.luau:347` | `penv_new_key` | vazio → "chave vazia" |
| `handlers/projects.luau:362` | `penv_new_val` (modo secret) | vazio → "escolha um secret" |
| `handlers/wizard.luau:217` / `new_service_window.luau:41` | `ns_name` | vazio → "informe um nome" |
| `handlers/secrets.luau:39,45` | `sec_new_name`, `sec_new_val` | vazio → "informe um nome/valor" |
| `handlers/services.luau:334` | `f_domain` | vazio → "domínio vazio" |
| `handlers/services.luau` (`erro_f_gen_port`) | `f_gen_port` | porta — hoje sem validação de faixa real |
| `handlers/connection.luau:198` | `url` | regex de URL na mão → "URL inválida" |
| `new_job_window.luau` | `njob_name`, `njob_hours` | nome vazio; horas via `<spinbox min max>` |

Todas viram uma linha de atributo:

```xml
<!-- antes: input + on_change="field:np_name" + if no handler -->
<input class="np_input" form_control="np_name"
       rules="required|minlen:1" msg="informe um nome" />
<text class="err" if="{erro_np_name}" not_empty>{erro_np_name}</text>
```

e o handler perde a escada inteira — `on_submit` **só roda quando tudo passou**;
`on_validation_error` recebe as falhas em JSON para o extra (toast, foco).
`validate_on="submit"` faz editar um campo apagar o erro dele sozinho — o que
**elimina a razão de existir do prefixo `field:`** (hoje ele serve para gravar a
chave e, em alguns handlers, limpar o `erro_`). Com `form_control` puro o motor
grava a chave; sem `field:` some uma indireção de todo formulário.

Regras que o vocabulário cobre: `required`, `minlen`/`maxlen`, `digits:N|MIN,MAX`
(feito para CPF/telefone — aqui serve para "porta tem 2–5 dígitos"),
`gte`/`lte` (faixa numérica: porta `gte:1|lte:65535`), `email`, `accepted`
(checkbox de termos), `fn:NOME` (escape hatch — URL do daemon, "nome de serviço
já existe", dígito de compose válido) e `pattern="…"` (regex à parte).

### 2.B — Sidebar: `@media` manual e item que apaga

`shell.gv` + `components/nav_item.gv` implementam à mão o que o `<drawer>` faz:

- dois breakpoints (`@media (max-width: 900|560)`) que encolhem `.sidebar` para
  um trilho de ícones e escondem rótulos — replicados em `nav_item.gv`;
- `nav_item.gv` tem **dois `<button>` irmãos** (`nav_row_on` / `nav_row_off`)
  com o conteúdo duplicado, porque classe condicional exigia isso antes;
- o item "Projects" usa `one_of="{target}"` com `target` multi-view
  (`"projects project_services service"`) para não apagar nas subtelas — hack
  documentado no próprio arquivo.

Com `<drawer value="menu" open="{menu}">`:

- `size`, `padding`, `duration`, `panel_class` são props — `.sidebar` no `.gss`
  fica **só com a cor** (é literalmente o que o CHANGELOG 0.101 diz do preset
  `minimo`);
- a gaveta empurra o conteúdo; um botão `☰` (`drawer::toggle:menu`) no header e
  `init()` semeando `menu = "true"`;
- em janela estreita, fecha a gaveta em vez de virar trilho — decisão de UX a
  tomar (ver Riscos), mas remove os dois `@media` e o `nav_item` dobrado;
- `class="nav_row_{on}"` já interpola (o motor faz isso desde sempre) →
  `nav_item.gv` vira **um** `<button>` com `class="nav_row_{estado}"`.

### 2.C — Tabelas: `thead` + `for-each` + `<hr>` por linha

Padrão em `home.gv` (Monitoring, Ingress, Deploy Engine ×3, Docker ×5),
`shell.gv` (Deployments, env, secrets, jobs) e `service.gv` (Deployments,
Domains). Cada uma é:

```xml
<row class="thead"><text class="th col_svc">SERVIÇO</text> … </row>
<scrollable><column class="table_body">
  <column class="row_wrap" for-each="deployments" var="d">
    <row class="trow"> …5–6 <text> por coluna… </row><hr />
  </column>
</column></scrollable>
```

`<tableview items="linhas" columns="colunas" value="sel" sort="ordem"
widths="larguras" virtualize="…">` entrega cabeçalho, ordenação por clique,
seleção, colunas arrastáveis e virtualização — e `columns` aceita **JSON inline**
(0.97), então some o `ctx.*_columns_json` publicado só pra isso. As células de
estado (`components/state_cell.gv`) viram `{ key:"estado", … }` com a cor no
`.gss` por classe `state_{kind}` (já é assim). ~15 blocos de ~25 linhas → ~15
tags + ~15 funções `publicar_*` que já existem, só mudando o shape.

Manter à mão: as listas kv arrastáveis (env do projeto/serviço) — têm
`on_reorder` + `drag_handle` + modo comentário, que `<tableview>` não cobre.

### 2.D — Abas: `<TabButton>` custom ×N

`components/tab_button.gv` (dois `<button>` irmãos, `equals="{target}"`
interpolado) é instanciado à mão em: `service.gv` (8 abas + provider Git/Zip),
`shell.gv` (4 sub-abas de projeto), `home.gv`/`docker` (5 sub-abas),
`new_job_window.gv` (compose/git, recorrência). Cada bloco é
`<row class="tabs"><TabButton …/> ×N</row>` + um handler `x_tab:y`.

`<TabBar items="abas" value="aba" active="{aba}" />` (só a barra, para quem já
tem o corpo condicionado) ou `<Tabs>` (barra + `<template slot="id">`). Os itens
`{id,label}` saem de um `ctx.*_tabs` — e o handler `x_tab:y` some (o `<TabBar>`
grava a chave sozinho). `<TabButton>` pode ser deletado.

### 2.E — Wizards: passos com `<if cond="{step}" equals>`

`new_service.gv` (`ns_step`: pick_kind → db_form / pick_template → template_form)
e `new_job_window.gv` (`njob_step`: pick_project → pick_service → form) montam
navegação, "voltar/avançar" e o gate de validação à mão.

`<Wizard steps="…" titles="…" value="passo" valid="{ok}" on_finish=… on_cancel=…>`
com cada passo num `<template slot="id">` dá cabeçalho de passos, botões e trava
o "avançar" quando `valid` é falso — casa com a validação declarativa da Onda 1
(`valid="{form_ok}"`, publicado pelo `on_validation_error`/`on_submit` do passo).

### 2.F — Chrome de janela repetido 5×

`app.gv`, `new_service_window.gv`, `new_project_form.gv`, `new_job_window.gv`,
`new_registry_token_window.gv`, `log_window.gv` cada um redeclara: a moldura de
6px de resize handles, a titlebar custom (`window:drag/minimize/maximize/close`),
e re-linka `theme.json` + `app.gss`. Isto **não** é buraco do glacier
(`decorations=false` obriga a moldura no markup — o preset `completo` faz igual),
mas pode virar **um** componente `<WindowChrome>` com `<slot/>` (a coleta de
`<dialog>` é recursiva; a de `<style>` global também) e um `<include>` da moldura.
Cada `*_window.gv` cai de ~40 linhas de boilerplate para ~5.

---

## 3. A reforma, em ondas

Ordem por dependência e por risco. Cada onda é um PR, fecha verde
(`cargo test -p rustploy-gui --test templates_render` + `--bins` + o
`luau-lsp analyze`) e é conferida na tela (`make` de screenshot headless — ver
AGENTS.md).

### Onda 0 — o bump (pré-requisito de tudo)

1. `glacier-ui = "0.87"` → `"0.102"` em `crates/rustploy-gui/Cargo.toml`.
2. **Antes**, o passo 0 do AGENTS.md ("nunca `path`, nunca `[patch]`"):
   `diff -rq` o tree local do glacier-ui contra o pacote publicado e `git fetch`
   nos dois sentidos — a 0.87 do rustploy pode estar atrás do que foi publicado
   de outra máquina.
3. `cargo check -p rustploy-gui` + `cargo test -p rustploy-gui`.
4. Quebras declaradas no intervalo a conferir (o estudo anterior mostrou que o
   rustploy passa longe da maioria):
   - `app:` é prefixo reservado de ação (0.63) — nenhuma ação nossa se chama assim.
   - `<button>` sem `padding` explícito herda o default do iced de novo (0.68) —
     18 das 19 classes de botão já declaram `padding`.
   - nomes registrados (`StatCard`, `TabButton`, `NavItem`, `StateCell`, …) não
     colidem com builtins novos (`drawer`, `tableview`, `wizard`, `autocomplete`).
   - `else-if` **existe** hoje (a nota "não existe else-if" no plano de
     convergência é de antes da 0.94) — as escadas `<if>` aninhado em `<else>` de
     `shell.gv`/`home.gv` podem achatar, mas isso é a Onda 3, não esta.
5. Nada de UI muda nesta onda. É só a porta.

**Feito quando:** os 58+ testes verdes, o app abre, screenshot idêntico ao de antes.

### Onda 1 — formulários no padrão declarativo *(a que "não pode faltar")*

Alvo, em ordem de tamanho:

1. **`new_project_form.gv`** — o menor, é o piloto. `np_name` →
   `rules="required|minlen:1" msg="informe um nome"`; `np_desc` sem regra.
   `submit_project` perde a escada; ganha `on_validation_error="np_apontar"`.
   Remove `on_change="field:np_name"`.
2. **`shell.gv`** — `edit_project` (`edit_proj_name` required), `penv_add`
   (`penv_new_key` required; no modo secret, `penv_new_val` required com
   `msg` trocada), `secret_add` (`sec_new_name` + `sec_new_val` required).
3. **`login.gv` / `connection.luau`** — `url` com `rules="required|fn:valid_url"`
   e `valid_url(v)` devolvendo a mensagem (move o regex de `connection.luau:198`
   para uma função nomeada, testável).
4. **`service.gv`** — form `general`: `f_repo_url` (`required` quando provider
   Git), `f_gen_port` (`rules="gte:1|lte:65535" msg="porta de 1 a 65535"` —
   corrige a ausência de validação de faixa de hoje); `domains`: `f_domain`
   (`required|fn:valid_domain`).
5. **`new_registry_token_window.gv`** — nome/escopo do token.
6. **`new_service.gv` / `new_job_window.gv`** — só os campos de texto agora
   (`ns_name`, `njob_name`, `njob_compose_path`, `njob_main_service`); a
   navegação de passos é a Onda 5. `njob_hours` já é `<spinbox min max>`.

Convenções desta onda (alinhar com o preset `formulario` do glacier-cli, que é o
padrão de referência):

- `error_prefix` fica no default `erro_` → os `{erro_<campo>}` do markup não mudam.
- `.campo:invalid { border_width: 1; border_color: var(--danger); … }` no
  `app.gss` — **um** bloco, e o motor acende sozinho. Hoje o rustploy não tem
  `:invalid`; ganha destaque visual de graça.
- `<button type="submit">` dentro do `<form>` dispara sem `on_click`; o
  `type="reset"` limpa os `erro_` antes de rotear o `on_click`.
- `fn:NOME` para o que o vocabulário não cobre: `valid_url`, `valid_domain`,
  `porta_livre` (checa contra as portas de host já alocadas — o daemon tem o
  dado no snapshot).

Impacto no Luau: as escadas de `if … erro_X … return` da §2.A saem; entram
funções `*_apontar(erros_json)` curtas (toast + foco). Saldo estimado:
**−150 a −250 linhas** em `handlers/{projects,services,secrets,connection}.luau`.

**Feito quando:** cada `<form>` envia só com tudo válido; `:invalid` acende;
`templates_render` cobre um caso de falha por form (o teste tem que olhar a
árvore avaliada — ver AGENTS.md "Antes de dizer que funciona").

### Onda 2 — sidebar com `<drawer>` *(o outro "tem que ter")*

1. `shell.gv`: a `<column class="sidebar">` vira
   `<drawer value="menu" open="{menu}" size="264" panel_class="sidebar">`,
   com os `<NavItem>` dentro do painel e o `<column class="main_col">` como
   conteúdo.
2. `nav.luau` / `app.luau` `init()`: semear `ctx.menu = "true"`.
3. Botão `☰` na `.topbar` com `on_click="drawer::toggle:menu"` (prefixo que o
   motor consome — sem handler).
4. `nav_item.gv` → **um** `<button class="nav_row_{estado}" on_click="{action}"
   tooltip="{label}">`; `estado` publicado pelo item ou derivado com
   `one_of="{target}"` mantido só para o "aceso nas subtelas".
5. `app.gss`: apaga `.sidebar { width/padding/spacing }` e os dois `@media` de
   sidebar em `shell.gv` **e** em `nav_item.gv`; sobra `.sidebar { background:
   var(--surface); }`.
6. Decisão de UX (ver Riscos): em `< 900px` a gaveta **fecha** (default do
   `<drawer>`) em vez do trilho de ícones. Se o trilho for requisito, ele volta
   como um `@media` no `panel_class` — mas aí o `nav_item` dobrado também volta,
   e metade do ganho evapora. Recomendação: adotar o fechar, com o `☰` sempre
   visível na topbar.

**Feito quando:** a gaveta abre/fecha animada, empurra o conteúdo, o item ativo
acende, e `app.gss` perdeu os `@media` de sidebar.

### Onda 3 — tabelas com `<tableview>`

Uma tela por PR (são independentes). Ordem: Monitoring → Ingress → Deployments
(shell) → Deploy Engine → Docker (5 sub-abas) → service.gv (Deployments/Domains).

Por tela: trocar o bloco `thead`+`for-each`+`<hr>` por
`<tableview items="X" columns='[…JSON inline…]' value="sel" virtualize="…">`;
a função `fmt/*.publicar_X` continua produzindo as **linhas** (mesmo array de
objetos), só larga o array paralelo de colunas. Achatam também as escadas
`<if cond="{tab}" equals>` que hoje aninham em `<else>` (o `else-if` da 0.94).

Ganho colateral: fecha a §2.C do plano de convergência (a webui já usa `<table>`
de verdade — as duas passam a ter a mesma árvore).

**Feito quando:** ordenação por clique funciona onde fazia sentido, a seleção
escreve a chave certa, e o teste olha a árvore avaliada da `<tableview>` (não só
`ctx.X` de ida e volta).

### Onda 4 — abas com `<TabBar>` / `<Tabs>`

`service.gv` (8 abas), `shell.gv` (sub-abas de projeto), `home.gv`/docker
(sub-abas), `new_job_window.gv`. `ctx.*_tabs = json.array({{id,label}, …})`;
`<TabBar items="svc_tabs" value="tab" active="{tab}" />`. Deleta
`components/tab_button.gv` e os handlers `*_tab:*`.

### Onda 5 — wizards com `<Wizard>`

`new_service.gv` e `new_job_window.gv`: `<Wizard steps titles value valid
on_finish on_cancel>` com cada passo em `<template slot>`. `valid="{passo_ok}"`
ligado à validação da Onda 1 (o `on_submit` do passo publica `passo_ok = "1"`).
`handlers/wizard.luau` perde a máquina de `ns_step` manual (~60–90 linhas).

### Onda 6 — chrome de janela num componente

`<WindowChrome title="…">` com a moldura de resize (via `<include>`) + titlebar +
`<slot/>`. Cada `*_window.gv` reimporta `theme`/`app.gss` (motor isolado, não
herda) mas o corpo vira `<WindowChrome title="Novo job — Rustploy"><…/></WindowChrome>`.
O teste `janelas_declaram_titulo_e_tamanho_no_proprio_template` continua valendo
(o `<screen title size>` fica; só o miolo é componentizado).

### Onda 7 — selects e autocomplete (polish, opcional)

Os `<select options labelField valueField>` de conta/repo/branch (Git) já são
idiomáticos. Onde a lista é longa e vem do servidor (`gitea_repos`), trocar por
`<autocomplete items filter="false">`. Baixa prioridade.

---

## 4. Paridade com a webui (obrigatória)

AGENTS.md: "Uma mudança de interface que só entre num dos dois vira divergência
silenciosa." Cada onda tem um par na webui:

| Onda | GUI (`views/`) | webui (`crates/daemon/webui/`) |
|---|---|---|
| 1 forms | `rules=` no `<form>` | validação declarativa equivalente — HTML5 `required`/`pattern` + um helper Alpine; ou portar a mesma DSL. **Decidir aqui**, não deixar divergir. |
| 2 drawer | `<drawer>` | `<aside>` com `x-show="$store.app.menu"` + transição CSS — a webui já tem o layout, falta o toggle |
| 3 tabelas | `<tableview>` | a webui **já** usa `<table>` real — esta onda *aproxima* as duas |
| 4 abas | `<TabBar>` | `screens/*.js` já têm `activeTab` — trocar o markup copiado por um partial |
| 5 wizard | `<Wizard>` | `x-show` por passo — já é plano na webui |

Gotcha recorrente da webui (AGENTS.md): filhos de `.scroll_fill` sem
`flex-shrink: 0` somem; `Cache-Control: immutable` por um ano → `ctrl+shift+r` ao
testar. Idealmente esta reforma e o `plano-convergencia-templates-gui-webui.md`
avançam juntos — as ondas 2–5 removem justamente a "divergência causada por
lacuna do glacier-ui" que aquele plano lista.

---

## 5. Riscos e armadilhas

- **`.luau` não tem hot-reload** (AGENTS.md). Toda onda que mexe em handler pede
  reiniciar o app — não confunda "não fez efeito" com bug.
- **`.luaurc` anula tipos**: `fetch`/`json`/etc. listados em `globals` viram
  `any`. As `fn:` novas (`valid_url`, …) declaram no `glacier.d.luau`, não no
  `.luaurc`.
- **Drawer em janela estreita**: perder o trilho de ícones é uma mudança de
  comportamento visível. Levar a decisão a você antes de codar a Onda 2.
- **`<tableview>` não faz reorder nem linha-comentário** — as listas kv de env
  ficam à mão de propósito.
- **`value` é nome de chave, não interpolação** — o erro mais caro do motor
  (AGENTS.md, "Quatro armadilhas que falham EM SILÊNCIO"). Revisar cada
  `value=`/`active=`/`items=` das tags novas.
- **Teste que passa com widget vazio**: os testes desta reforma têm que olhar a
  **árvore avaliada** (ramo existe? `value_var` é a chave certa? largura é número
  ou `fill` dentro de `shrink`?), não escrever `ctx.X` e ler de volta.
- **`fill` dentro de `shrink` colapsa** — a cadeia inteira até o campo de um
  `<form>`/`<drawer>` precisa ser `width: fill` (a "armadilha de sempre" do
  DIALOGS.md vale aqui também).
- **Divergência glacier local × publicado** (o passo 0 do AGENTS.md): fazer o
  `diff -rq` antes do bump, senão a Onda 0 pode reverter versões em silêncio.

---

## 6. Não-objetivos

- Reescrever `service.gv`/`home.gv` do zero — as ondas são incrementais,
  tela a tela, cada uma reversível.
- Adotar `<dock>`, `<canvas>`, `<mdiarea>`, `FontSelect` — nada no rustploy pede.
- Mexer no protocolo, no daemon ou na API de agente.
- `[patch.crates-io]` ou dependência por `path` no glacier-ui — sempre publicar
  e subir a versão (AGENTS.md).

---

## 7. Ordem sugerida de execução

```
Onda 0  bump 0.87→0.102 ....................... 1 PR, meio dia
Onda 1  forms declarativos ................... 4–6 PRs (1 por grupo de tela)
Onda 2  drawer na sidebar ................... 1 PR  (+ decisão de UX antes)
Onda 3  tableview .......................... 6–7 PRs (1 por tela, paralelizável)
Onda 4  tabbar ............................. 2–3 PRs
Onda 5  wizard ............................. 2 PRs
Onda 6  window chrome ...................... 1 PR
Onda 7  autocomplete ...................... opcional
```

Ondas 1 e 2 são as pedidas explicitamente e podem ir primeiro, logo após a 0.
As 3–5 são independentes entre si e da 1–2 (só dependem da 0). A 6 é cosmética e
pode ficar para o fim.

Estimativa de saldo de linhas ao fim das ondas 1–5:
`views/` **−1.200 a −1.800 `.gv`**, Luau **−400 a −600**, `.gss` **−60 a −100**
(os `@media` e os `<style>` inline de tabela/aba/sidebar).
