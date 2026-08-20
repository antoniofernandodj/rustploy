# Plano: convergência dos templates (GUI iced ↔ web UI)

Estado: **proposta** (nada implementado ainda).
Escopo: `crates/rustploy-gui/views/**` (glacier-ui, `.gv` + `.gss` + Luau) e
`crates/daemon/webui/**` (HTML + Alpine + CSS + JS), mais as mudanças
necessárias no `glacier-ui` (`~/Desenvolvimento/glacier-ui`).

---

## 1. Ponto de partida: elas já são mais parecidas do que parecem

A web UI não é uma UI independente — é uma **tradução literal** da UI iced,
feita à mão. Isso está explícito no cabeçalho de `webui/app.css` ("tradução
literal dos design tokens e classes de `views/styles/app.gss`") e em cada
`screens/*.js` ("porta de `fmt/dashboard.luau`").

O que já converge hoje:

- **As mesmas tags**: o CSS declara `row`/`column`/`text` como elementos
  customizados (`display:flex`) só para o HTML ficar parecido com o `.gv`.
- **Os mesmos nomes de classe** (`thead`, `trow`, `td_key`, `col_svc`,
  `nav_row_on`, `stat_card`, `state_dot`…) e os mesmos design tokens.
- **A mesma árvore**. Lado a lado, a tabela de Deployments
  (`shell.gv` ~L170 vs `index.html` ~L156) tem elemento por elemento a mesma
  estrutura; a diferença é só o dialeto de diretiva.
- **Os mesmos nomes de módulo/função** na camada de lógica (`fmt.deployments`
  ↔ `fmt.js`, `handlers/projects.luau` ↔ `screens/projects.js`).

Ou seja: o trabalho **não** é aproximar duas UIs diferentes, é eliminar a
divergência de *dialeto* e a *duplicação* que sobrou. Volume atual:

| Lado | Markup | Estilo | Lógica |
|---|---|---|---|
| iced (`views/`) | 4.960 linhas `.gv` (10 componentes reutilizáveis) | 220 linhas `.gss` + `<style>` por template | 6.539 linhas Luau |
| web (`webui/`) | 2.144 linhas `index.html` (0 componentes) | 385 linhas `.css` | 3.668 linhas JS |

Os 10 componentes contra 0 explicam boa parte da diferença de tamanho do
markup: no HTML os 7 itens de sidebar, os cards de stat, as células de estado
e os botões de aba estão copiados e colados.

---

## 2. Mapa da divergência

### 2.A — Divergência acidental (mesma semântica, sintaxe diferente)

Tem tradução mecânica, nos dois sentidos:

| Conceito | `.gv` (glacier) | `index.html` (Alpine) |
|---|---|---|
| interpolação | `{d.service}` no corpo do nó | `x-text="d.service"` |
| condicional (elemento) | `<if cond="{view}" equals="x">…</if><else>…</else>` | `x-show="$store.app.view === 'x'"` |
| condicional (atributo) | `<text if="{msg}" not_equals="" …>` | `x-show="store.deploymentsMsg"` |
| laço | `<column for-each="deployments" var="d">` | `<template x-for="d in rows" :key="…">` |
| campo de texto | `<input value="search" on_change="search_changed"/>` | `x-model` + `@input="…"` |
| ação | `on_click="stop_all"` (nome de função) | `@click="store.stopAll()"` (expressão) |
| separador | `<rule />` | `<hr class="rule" />` |
| rolagem | `<scrollable direction="vertical">` | `<column class="scroll_fill">` (CSS `overflow`) |
| componente | `<link rel="import" … as="NavItem"/>` + `<NavItem label="…"/>` | copiar/colar o bloco |

### 2.B — Divergência **causada por lacuna do glacier-ui**

Aqui a web UI não divergiu por gosto: divergiu porque o `.gv` não expressa a
coisa. São as que valem "adaptar o glacier-ui", como você já autorizou.

1. **`<button>` não aceita filhos.** Por isso o item de sidebar virou uma
   `<row on_press>` no iced (está documentado em `components/nav_item.gv`) e
   um `<button>` de verdade no web. Além do dialeto, isso custa foco/teclado
   e semântica de botão no desktop.
2. **Não existe `else-if`.** As cadeias de tela em `shell.gv`/`home.gv` são
   `<if>` aninhado dentro de `<else>`, indentando mais um nível por tela; no
   web é uma lista **plana** de `x-show`. Sozinho, isso já torna os dois
   arquivos estruturalmente incomparáveis.
3. **Não existe condição composta.** A sidebar do web usa
   `['projects','project','new_service','service'].includes(view)` para manter
   "Projects" aceso nas subtelas. O `.gv` não tem como — e por isso, no client
   iced, **o item apaga** quando você entra num projeto. Não é só sintaxe: é
   uma diferença de comportamento visível.
4. **Classe condicional exige dois nós irmãos** (`nav_item.gv`,
   `stat_card.gv`, `state_cell.gv`: um nó `if` + um nó `else`, com todo o
   conteúdo duplicado), contra `:class="cond ? 'a' : 'b'"` no web. Detalhe
   importante: o `class` do glacier **já interpola** (`eval.rs:1177`), então
   `class="nav_row_{on}"` já funciona hoje — falta só adotar a convenção.
5. **`href` de `<link rel="import">` é resolvido pelo CWD**, e por isso cada
   `.gv` carrega o caminho absoluto-do-workspace
   (`crates/rustploy-gui/views/components/nav_item.gv`) dentro de si. O
   `require` do Luau já resolve relativo ao arquivo desde a 0.22; o `<link>`
   não. Enquanto for assim, servir o mesmo `.gv` por HTTP exige reescrever
   caminho.
6. **Sem teste de lista vazia**: o Luau precisa publicar um `*_count` só para
   o `<text if="{eng_queued_count}" equals="0">`; o web usa
   `rows.length === 0` direto.
7. **Sem aliases** de `<hr>`/`<rule>` (o parser já tem o mecanismo de alias:
   `if`/`se`, `on_change`/`on-change`…).

### 2.C — Divergência estrutural (é decisão de arquitetura, não sintaxe)

1. **`<if>` desmonta a subárvore; `x-show` só esconde.** As 12 telas do web
   estão *sempre* no DOM. Converger o dialeto muda isso (vira
   `<template x-if>`) — é a mudança de comportamento mais arriscada do plano.
2. **Contrato de dados.** O glacier tem um contexto **plano de strings**
   (`ctx.deployments = <JSON>`, `ctx.proj_env_count = "3"`), preenchido por
   *push* a cada snapshot (`handlers/stream.luau`). O web tem um store com
   objetos e **getters reativos** que recalculam sob demanda a partir de
   `store.snap`.
3. **Duas portas da mesma lógica** (Luau vs JS) que nada obriga a andar
   juntas. Hoje a paridade é mantida por disciplina e comentário de cabeçalho.
4. **Multi-janela**: 10 chamadas `open_window` e 5 `.gv` de janela
   (`new_project_form`, `new_job_window`, `new_service_window`,
   `new_registry_token_window`, `log_window`) contra formulários inline no web.
5. **Cromo só-desktop**: titlebar borderless, 8 handles de resize, bandeja.
   E cromo só-web: PWA, service worker, `localStorage`.

---

## 3. Direção proposta

> **O `.gv` é o dialeto canônico. A web passa a consumir `.gv`. O glacier-ui
> ganha o que falta para o dialeto servir aos dois.**

Por quê nessa direção:

- O `.gv` é o **mais restrito** dos dois. Tudo que ele expressa tem tradução
  direta para DOM; o contrário é falso (não dá para colocar uma expressão JS
  dentro do motor iced sem inventar um interpretador de expressões).
- O custo é assimétrico: um runtime `.gv` no browser é da ordem de **500–700
  linhas** de JS; o caminho inverso é reescrever o motor de avaliação do
  glacier.
- O `AssetSource` do glacier (seam de assets, já existente) e o `build.rs` do
  daemon (que já embute e gzipa `webui/`) dão o encanamento para servir o
  **mesmo** arquivo `.gv` para os dois consumidores sem I/O em runtime.

O que **não** é objetivo: fundir os dois runtimes, nem fazer o iced renderizar
HTML. São dois renderizadores; o que passa a ser único é o *template*, o
*estilo* e (fase 5, opcional) a *lógica*.

### Alternativas descartadas

- **Manter dois dialetos e criar um linter de paridade** (comparar árvores
  `.gv` × HTML e falhar no CI). Barato, mas conserva a duplicação: toda tela
  nova continua sendo escrita duas vezes. Resolve o *drift*, não o *custo*.
- **Adotar o dialeto Alpine no `.gv`** (`x-show`, expressões). Exige um
  avaliador de expressões dentro do glacier — muito mais caro que o inverso, e
  degrada o rastreamento de dependências (`Reads`/`EvalCache`), que hoje sabe
  exatamente quais chaves cada subárvore lê.
- **Gerar `.gv` a partir do HTML** (HTML como fonte). Mesmo problema da
  anterior, com o agravante de o HTML ser o lado sem componentes.

---

## 4. Plano por fases

Cada fase é independentemente entregável e deixa o repositório funcionando.
As fases 1–3 já eliminam a maior parte da divergência **sem** tocar no
renderizador do browser; a fase 4 é a que unifica os arquivos.

### Fase 0 — Rede de segurança e inventário

**Objetivo:** poder mudar as duas UIs sabendo se elas ainda mostram a mesma
coisa.

- Inventário de paridade tela a tela: telas, sub-abas, ações e campos que
  existem de um lado e não do outro. Já se sabe de algumas (`support` só no
  iced; o item "Projects" que apaga nas subtelas; as 5 janelas separadas).
- Estender `crates/rustploy-gui/tests/templates_render.rs` para cobrir também
  as telas que hoje não são exercitadas.
- Criar o equivalente web: um teste headless que carrega `index.html` com um
  snapshot fixo e valida que cada tela renderiza (via `jsdom`/Playwright, ou —
  preferível, para não trazer Node ao build — um teste Rust que sirva os
  assets e dirija um browser headless já disponível).
- **Pronto quando:** existe um comando único que valida as duas UIs e a lista
  de divergências de conteúdo está escrita.

**Custo:** 1–2 dias. **Risco:** baixo.

### Fase 1 — glacier-ui 0.58: fechar as lacunas da categoria 2.B

Cada item abaixo é pequeno e isolado; todos seguem a receita de publicação do
`CLAUDE.md` (conferir tree × versão publicada → mudar → bump → `cargo publish`
→ subir a dep → `cargo check` → **push**).

1. `<button>` com filhos (mantendo `text="…"` como atalho). Remove o hack de
   `nav_item.gv` e alinha com o HTML.
2. `<else-if cond="…" equals="…">`, encadeando com o `last_if` que
   `expand_children` já mantém. Aplaina `shell.gv` e `home.gv`.
3. Condição em conjunto: `one_of="projects project new_service service"`
   (ou `equals_any`). Evita inventar gramática de expressão — é uma
   comparação contra uma lista separada por espaço.
4. `empty` / `not_empty` para chaves de lista (a chave já é um array JSON no
   contexto). Aposenta os `*_count` que só existem para o estado vazio.
5. Aliases de tag: `<hr>` → `Rule`, e o que mais o inventário apontar.
6. `href` de `<link rel="import">` resolvido **relativo ao arquivo**
   importador (como o `require` do Luau desde a 0.22), com o caminho absoluto
   ainda aceito por compatibilidade.
7. (Opcional, mas resolve 2.C.5) `<if platform="desktop">` / `platform="web"`
   — deixa a titlebar/resize handles e o cromo de PWA convivendo no **mesmo**
   arquivo em vez de forçar dois arquivos.

Depois de publicada a versão, aplicar no rustploy: adotar `class="nav_row_{on}"`
nos três componentes de classe condicional, aplainar as cadeias de `if`, e
acertar o item "Projects" com `one_of`.

**Pronto quando:** `cargo test -p rustploy-gui --test templates_render` passa e
o diff `shell.gv` × `index.html` é lido lado a lado sem nós extras de um lado.

**Custo:** 3–5 dias (a maior parte é o item 1, que mexe em `widget.rs`).
**Risco:** médio — é a crate que sustenta o app inteiro; cada item deve sair
numa versão própria, não num bump só.

### Fase 2 — Fonte única de estilo: `.gss` → `.css` gerado

**Objetivo:** apagar `webui/app.css` como arquivo mantido à mão. Ele passa a
ser **gerado** de `views/styles/app.gss` (mais os `<style>` de cada template)
pelo `build.rs` do daemon, que já lê, minifica, gzipa e embute os assets.

Tradução (o GSS já tem classes, ids, tags, `:hover/:focus/:active/:disabled`,
`@media` e `var()` — o mapeamento é quase 1:1):

| GSS | CSS |
|---|---|
| `width: fill` / `height: fill` | `flex: 1 1 auto; min-width/height: 0` |
| `padding: 11 14` | `padding: 11px 14px` |
| `spacing: 10` | `gap: 10px` |
| `size: 13` / `bold: true` | `font-size: 13px` / `font-weight: 700` |
| `border-width` + `border-color` | `border: Npx solid …` |
| `hidden: true` | `display: none` |
| `@media (max-width: 900)` | idem |

O ponto espinhoso: **`align-x`/`align-y` dependem de o nó ser `row` ou
`column`** (em `row`, `align-y:center` é `align-items`; em `column`, é
`justify-content`). Solução sem ambiguidade: o compilador emite as **duas**
variantes qualificadas pela tag —
`row.x { align-items:center } column.x { justify-content:center }`.

Segundo ponto: os `<style>` de template são **escopados** no glacier e globais
no CSS. O compilador emite tudo global e **falha o build** se duas classes de
mesmo nome forem declaradas com corpos diferentes em templates distintos.

**Pronto quando:** `app.css` sai do git, o build o gera, e a comparação visual
das telas não muda.

**Custo:** 3–4 dias. **Risco:** médio (regressão visual). Mitigação: manter o
`app.css` antigo por um commit e diferenciar o gerado contra ele.

### Fase 3 — Contrato de dados único

**Objetivo:** o web passa a ter o mesmo formato de estado do glacier, o que é
pré-requisito para a fase 4 e, por si só, já torna as duas camadas de lógica
comparáveis linha a linha.

- O `Alpine.store('app')` vira um **`ctx` plano** com exatamente as mesmas
  chaves que o Luau escreve (`deployments`, `proj_env_count`, `daemon_uptime`,
  `services_label`…), preenchido por *push* no mesmo ponto em que o Luau
  preenche (`handlers/stream.luau` ↔ o handler de snapshot do web). Adeus
  getters reativos por tela.
- `screens/*.js` deixam de ser componentes Alpine e viram
  `handlers/*.js`: um **registro de ações por nome** (`stop_all`,
  `nav_projects`, `open_new_project_window`), espelhando `handlers/*.luau`.
  É o que permite `on_click="stop_all"` funcionar nos dois lados.
- `fmt.js` é quebrado nos mesmos módulos de `fmt/` (`time`, `dashboard`,
  `service_detail`, `jobs`, `git`, `registry`, `docker_cleanup`, `util`).
- **Fixtures compartilhadas**: um diretório de snapshots JSON de entrada e
  saídas esperadas, consumido pelo teste Luau **e** pelo teste JS. É isso que
  passa a garantir a paridade da lógica de formatação, em vez de disciplina.

**Pronto quando:** as fixtures rodam nos dois lados com o mesmo resultado, e
nenhuma tela do web lê `store.snap` diretamente.

**Custo:** 5–8 dias (é a fase mais braçal). **Risco:** médio-alto — é
reescrever a orquestração do web. Mitigação: tela por tela, com a fase 0
validando cada passo.

### Fase 4 — Runtime `.gv` no browser e template único

**Objetivo:** `index.html` deixa de conter markup de tela; as telas passam a
ser os **mesmos arquivos** `views/*.gv` do client iced.

Arquitetura do runtime (`webui/gv.js`, estimado em 500–700 linhas):

1. **Carregar e parsear.** Separar o bloco `<script>` (espelhando
   `eval.rs::strip_script`), embrulhar o resto num nó raiz sintético e parsear
   com `DOMParser` em modo **`application/xml`**. É obrigatório ser XML, não
   HTML: o parser HTML **baixa a caixa** dos nomes (`<NavItem>` → `navitem`) e
   **ignora o self-closing** de elementos desconhecidos (`<StatCard/>` viraria
   uma tag aberta engolindo o resto da tela). Os `.gv` já são XML válido — o
   `roxmltree` os parseia hoje.
2. **Resolver `<link rel="import">`** recursivamente (com o `href` relativo da
   fase 1) e montar o registro de componentes.
3. **Transpilar para DOM Alpine** (não escrever um motor reativo novo):
   `{x}` → `x-text`; `<if>`/`<else-if>`/`<else>` → `<template x-if>` com a
   condição já traduzida; `for-each`/`var` → `<template x-for>`;
   `on_click="f"` → `@click="act('f')"`; `value="k"` → `x-model="ctx.k"`;
   `<rule>` → `<hr>`; `<scrollable>` → `column.scroll_fill`; uso de
   componente → subárvore do template com as props numa camada de escopo.
4. **Servir os `.gv`** pelo `build.rs` do daemon, que já embute e gzipa —
   basta incluir `crates/rustploy-gui/views/**` na varredura.

Migração **incremental** e segura: o runtime sabe renderizar um `.gv` dentro
de um contêiner, então dá para migrar tela por tela, com as telas ainda não
migradas continuando em HTML no `index.html`, que vai encolhendo.

Atenção ao ponto 2.C.1: trocar `x-show` por `x-if` **desmonta** a tela ao sair
dela. Como na fase 3 todo o estado passou a viver no `ctx`, isso é o
comportamento correto (é o do client iced) — mas é o item a olhar com lupa nos
testes.

**Pronto quando:** `index.html` não tem mais markup de tela e as telas do web
são carregadas de `views/*.gv`.

**Custo:** 8–12 dias. **Risco:** alto, mas contido pela migração incremental.

### Fase 5 (opcional, avaliar depois da 4) — Luau no browser

Com template, estilo e contrato de dados unificados, o que sobra duplicado é a
**lógica** (`views/scripts/*.luau` ↔ `webui/handlers/*.js`). O fim de linha é
compilar o Luau para WASM e rodar os **mesmos** `.luau` no browser, com um
prelúdio JS implementando `fetch`, `sse`, `ctx`, `toast`, `storage` e
`open_window` (este último como modal).

Não faz parte do compromisso deste plano: só faz sentido decidir depois de
medir quanto de divergência realmente sobrou na fase 4, e o custo (binário
WASM de ~1–2 MB numa UI que hoje pesa poucas dezenas de KB) precisa ser pesado
contra o ganho.

---

## 5. O que fica divergente de propósito

- **Janelas separadas ↔ modais.** A web não tem multi-janela. As 5 janelas
  viram overlay renderizando o mesmo `.gv`, com `open_window`/`close_window`/
  `broadcast` reimplementados como abrir/fechar/emitir no mesmo documento — o
  contrato do template não muda.
- **Cromo de plataforma**: titlebar borderless, handles de resize e bandeja
  não existem no browser; PWA/service worker não existem no desktop. Ficam
  isolados por `<if platform="…">` (fase 1, item 7) no mesmo arquivo.
- **Persistência**: `storage` do glacier (disco) × `localStorage`. Mesma API
  do ponto de vista do template.

## 6. Riscos principais

1. **Regressão visual silenciosa** na fase 2 — a tradução GSS→CSS erra num
   canto e ninguém vê. Mitigação: diff contra o `app.css` atual, tela a tela.
2. **`x-if` no lugar de `x-show`** (fase 4) expõe qualquer estado que hoje
   dependa de o nó continuar montado. A fase 3 é o pré-requisito que remove
   essa dependência — não inverter a ordem.
3. **Bumps do glacier-ui em cascata** (fase 1): cada mudança é uma versão
   publicada, e o passo 0 da receita (conferir o tree local contra o pacote
   publicado) já falhou antes. Uma versão por item, com push imediato.
4. **Escopo de classe** (fase 2): colisão entre `<style>` de templates
   diferentes. Mitigada por falha de build, não por convenção.

## 7. Ordem sugerida

Fase 0 → 1 → 2 → 3 → 4, nessa ordem. As fases 1 e 2 são independentes entre
si e podem andar em paralelo; a 3 depende da 1; a 4 depende de 1, 2 e 3.

Corte natural se o orçamento apertar: **parar depois da fase 3**. Nesse ponto
o estilo e o contrato de dados já são únicos e os dois markups são
comparáveis nó a nó — sobra a duplicação do markup, mas sem divergência
semântica. As fases 4 e 5 é que trocam "comparável" por "o mesmo arquivo".
