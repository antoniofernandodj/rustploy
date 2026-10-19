# Plano (preliminar): tirar o glacier-ui de dentro do iced, sem reescrever o que é difícil

> Status: **proposta, nada implementado** (2026-10-19). Vive só na `main` do agregador; não há branch nem
> código em nenhum repositório. Os números foram medidos no `glacier-ui` 0.122.4 e no `rustploy-gui` 0.1.8
> desta data. As afirmações sobre bibliotecas de fora (Vello, Parley, Slint, egui, Taffy…) são **hipóteses a
> confirmar nos spikes da Fase 4**: o plano diz exatamente o que medir, justamente para não decidir de ouvido.
>
> Origem: uma conversa sobre "reescrever tudo em wgpu + winit puros e tirar o iced". A conclusão foi que
> reescrever o *renderizador* é o caminho caro e de pouco retorno, e que existem dois caminhos melhores,
> descritos aqui: **A** (o glacier-ui passa a mandar no próprio layout e nos próprios eventos, e o iced vira só
> a camada de desenho e entrada) e **B** (trocar essa camada por outra que já traga texto, acessibilidade e
> layout). O A é pré-requisito do B: depois dele, trocar de backend é escrever *um adaptador*, não refazer o toolkit.

## 1. O problema, em português simples

O `rustploy-gui` não fala com o iced. Ele fala com o **glacier-ui**, um toolkit nosso (61 mil linhas, estilo
"Qt em Rust") em que a tela é escrita em markup (`.gv`/`.gvb`), o estilo em `.gss` e a lógica em Luau. O
glacier-ui, por baixo, **monta uma árvore de widgets do iced** e deixa o iced calcular o layout, desenhar e
tratar o mouse e o teclado.

Imagine um tradutor simultâneo: você escreve numa linguagem rica (flex, `fill-portion`, `max-size`, `@media`,
seletores compostos) e o tradutor tem que dizer tudo numa linguagem mais pobre (o modelo de caixas do iced). O que
não tem tradução vira **gambiarra** ou **limitação**. Os sintomas que já aparecem no projeto:

- O motor `.gss` tem limitações documentadas (sem seletor por vírgula, sem `var()`, sem pseudo-estados em
  ancestral, `max-size` e `fill-portion` só entraram aos poucos).
- O drag-and-drop de listas foi feito **reescrevendo o JSON do contexto** a cada movimento, porque o iced não dá
  o gancho certo (`onReorder`/`reorderKey`).
- Há 7 widgets próprios escritos direto contra a API `advanced` do iced (`anchored`, `grid`, `reveal`,
  `overflow`, `smooth_scroll`, `animated_toggler`, `spinner`) só para fazer o layout e o evento que o iced não
  oferece.
- Cada salto de versão do iced obriga a mexer em 47 arquivos do glacier-ui, e o `rustploy-gui` carrega um
  `vendor/iced_tiny_skia` com correção de renderizador que o upstream ainda não lançou (`TODO.md`).

O que **não** é problema: o parser, o avaliador, o motor de estilo, o Luau, o gerenciamento de janelas, a bandeja e o
resto da lógica já são, em grande parte, independentes do iced. O acoplamento está concentrado na fronteira.

## 2. O que existe hoje (medido)

| Fato | Número / onde |
|---|---|
| Tamanho do glacier-ui | ~61 000 linhas; `parser.rs` 8 353, `widget.rs` 7 458, `lib.rs` 6 136, `luau/mod.rs` 4 802, `eval.rs` 4 276 |
| Arquivos que citam o iced | 47 de 82; os pesados: `lib.rs` (228 refs), `widget.rs` (138), `daemon.rs` (46), `anchored.rs` (34), `textarea.rs` (27), `grid.rs` (24) |
| Widgets implementados direto na API `advanced` do iced | 7 (`impl Widget<…> for`) |
| Onde a árvore vira widget | `widget.rs::render_node` (um `match` sobre `NodeType`) |
| Mensagens do motor | `widget.rs::EngineMessage`, ~45 variantes; é o ponto onde evento de widget vira ação |
| Renderers hoje | `tiny-skia` (software) **e** `wgpu`, com queda automática para CPU quando o adaptador não é Vulkan-compliant |
| Plataformas do glacier-ui | Linux (X11/Wayland), Windows, **Android** (feature `android`), **Web/wasm** (`web-gpu`), mais `webview` (wry), `tray` e MicroPython |
| Uso do iced no `rustploy-gui` | 2 linhas em 2 arquivos (`src/main.rs`, `src/app/mod.rs`); o resto é `.gv`/`.gss`/Luau |

A última linha é a que importa: **o rustploy não precisa mudar** para nada disto. O alvo é o glacier-ui.

## 3. Princípios (as regras do jogo)

1. **O contrato com o app não muda.** `.gv`/`.gvb`, `.gss`, API Luau, `GlacierDaemon`/`GlacierUI` e os nomes dos
   widgets continuam. Quem usa o glacier-ui sobe de versão e nada quebra; um comportamento só muda se for
   correção declarada no CHANGELOG.
2. **Não reescrever o que é difícil e já resolvido por outro.** Texto (shaping, bidi, quebra de linha,
   fallback de fonte), IME, clipboard e acessibilidade são *anos* de bug dos outros. O plano os **mantém atrás
   de uma biblioteca**, nunca os reimplementa.
3. **Fallback sem GPU é requisito.** Uma ferramenta de administração tem que abrir em VM, RDP e máquina velha.
   Hoje isso vem do `tiny-skia`. Backend que não tenha equivalente fica fora, ou entra só como opção.
4. **Cada fase sai usável sozinha** e dá para parar depois de qualquer uma (como no plano do daemon).
5. **Mover antes de mudar.** Uma fase que reorganiza código não muda comportamento no mesmo commit.
6. **Medir antes e depois.** Nada de "ficou melhor" sem número (seção 8).
7. **Regra do projeto vale aqui:** o `glacier-ui` nunca é `path`; é publicado no crates.io e o `rustploy-gui`
   sobe por versão. Um crate novo do glacier entra na mesma regra (ordem de publicação na seção 7).

## 4. Caminho A — o glacier-ui passa a mandar no layout e nos eventos

### 4.1 A ideia

Hoje: `parser → eval → render_node → árvore de widgets do iced → (iced) layout, desenho, eventos`.

Alvo, em camadas, **de baixo para cima só**:

```
 .gv/.gss/Luau ──► glacier-core ───────────────────────────────┐
   parser, eval, estilo (GSS), scripts, contexto, EngineMessage │  não conhece iced
   + árvore de visão (IR) + layout + eventos normalizados        │
                                                                ▼
                                                    trait Backend  (desenho + entrada)
                                                     │         │
                                              glacier-iced   (futuro) glacier-<outro>
```

- **Árvore de visão (IR).** Uma estrutura própria, pequena e estável, que descreve *o que desenhar*: caixas,
  texto, imagem, SVG, canvas, campos de edição. É o que `render_node` já produz, só que sem tipos do iced.
- **Layout próprio.** Flex e grid de verdade, calculados pelo glacier-core (ver 4.3). O iced deixa de decidir
  tamanho e posição.
- **Eventos normalizados.** Ponteiro, teclado, foco, rolagem, IME e janela entram no core como eventos nossos, e
  o core decide o que cada um vira (`EngineMessage`). O drag-and-drop passa a ser nativo, não JSON reescrito.
- **Backend.** O iced fica atrás de uma interface pequena: "desenhe esta lista de comandos", "meça este texto",
  "entregue-me os eventos". É o adaptador `glacier-iced`.

### 4.2 Estratégia de migração: de dentro para fora, sem big bang

O erro clássico aqui seria tentar trocar tudo de uma vez. O caminho seguro é **ir tomando território do iced
um pedaço por vez**, com o iced ainda dono da janela e dos campos de edição:

1. **Primeiro o layout**, implementado como **um widget contêiner do iced** (o `grid.rs` e o `anchored.rs` já
   provam que isso funciona: um `Widget` que mede e posiciona filhos `Element` por conta própria). Esse
   contêiner "GlacierFlex" aplica a semântica do GSS e substitui `row`/`column`/`container` um a um.
2. **Depois a IR e a fronteira.** Com o layout já nosso, `render_node` passa a produzir a IR, e um conversor
   IR → iced fica no adaptador.
3. **Por último a entrada.** Eventos passam a ser normalizados no core; os campos de texto continuam sendo
   widgets do iced (ilhas) até a Fase 4 decidir o destino.

Isso entrega valor desde a primeira fase (as limitações do GSS caem) sem ficar um mês com o app quebrado.

### 4.3 O motor de layout: usar o Taffy em vez de escrever um

O layout flex/grid é a parte que mais se parece com "reescrever o difícil". Hipótese a validar na Fase A1:
usar o **Taffy** (biblioteca Rust de layout CSS flexbox/grid/block, sem desenho nem texto; é o que vários
frameworks usam por baixo). Vantagens esperadas: semântica de flex *igual à do CSS* (o GSS já é inspirado nele),
`gap`, `wrap`, `min/max`, `aspect-ratio`, grid com áreas, tudo testado por terceiros. O que o glacier-core
precisaria dar é uma **função de medição de texto** (o Taffy pergunta "quanto mede este texto em tal largura?").
Critérios de aceite do Taffy: (a) reproduz o layout atual de todas as telas do rustploy dentro de uma tolerância
de 1px, (b) sem regressão de tempo de layout nas telas grandes (lista de serviços, logs), (c) licença compatível.
Se falhar, o plano de contingência é um flex próprio mínimo (só o subconjunto que o GSS expõe).

### 4.4 Fases

Cada fase tem **entrega**, **pronto quando** e **risco**. P = pequena, M = média, G = grande.

| Fase | O que | Esforço | Risco |
|---|---|---|---|
| A0 | Inventário e rede de segurança (medir tudo, congelar o comportamento atual) | P | baixo |
| A1 | Spike do layout: `GlacierFlex` com Taffy, lado a lado com o atual | M | médio |
| A2 | Layout próprio em produção: `GlacierFlex` substitui `row`/`column`/`container`/`stack` no `render_node` | G | **alto** |
| A3 | Extrair o `glacier-core`: tipos próprios (cor, comprimento, padding, alinhamento) no lugar dos do iced | G | médio |
| A4 | A IR e o `trait Backend`; `glacier-iced` vira o adaptador | G | médio |
| A5 | Eventos normalizados e drag-and-drop nativo | M | médio |
| A6 | Widgets próprios (os 7 `advanced`) reescritos sobre a IR/Backend | M | médio |
| A7 | Publicação, migração do `rustploy-gui`, remoção do `vendor/iced_tiny_skia` quando possível | P | baixo |

#### Fase A0 — Inventário e rede de segurança

- **Medir a linha de base** (seção 8) no `rustploy-gui` atual: RAM idle, tamanho do binário release, tempo até a
  primeira janela, tempo de layout de cada tela grande, uso de CPU rolando a lista de logs.
- **Classificar cada uso do iced** em `docs/` do glacier-ui: (1) tipo de dado (`Color`, `Length`, `Padding`,
  `Alignment`, `Font`), (2) widget pronto do iced (`button`, `text_input`, `scrollable`…), (3) widget próprio
  `advanced`, (4) runtime (`Task`, `Subscription`, janelas), (5) renderer. A tabela diz o tamanho real da
  fronteira — e provavelmente mostra que (1) e (4) são a maior parte.
- **Congelar o comportamento:** estender a técnica que o `AGENTS.md` já pede ("despejar a árvore avaliada de
  cada view/aba e comparar antes/depois") para um **snapshot de layout**: para cada tela e aba do rustploy, num
  viewport fixo, gravar o retângulo de cada nó (JSON). Esses arquivos viram o gabarito das fases A2 e A3.
- **Snapshot de imagem:** renderizar cada tela com o `tiny-skia` sem janela (headless) e guardar o PNG como
  referência visual; comparação por diferença de pixels com tolerância.
- **Pronto quando:** os snapshots rodam em CI (`cargo test`) e existe uma tabela de linha de base commitada.

#### Fase A1 — Spike do layout

- Construir `GlacierFlex` (um `Widget` do iced) cujo layout é delegado ao Taffy; mapear as propriedades do GSS
  (`direction`, `gap`, `padding`, `align`, `justify`, `grow`/`shrink`/`basis`, `min/max-size`, `fill-portion`,
  `wrap`) para as do Taffy.
- Escrever uma tela de teste que usa só `GlacierFlex` e comparar com o snapshot da A0.
- Registrar **cada diferença** de semântica entre o `Length::Fill/Shrink/FillPortion` do iced e o flex do
  Taffy (é onde mora o risco: o iced tem regras próprias de `Shrink` em botões — ver o fix `0.122.3`).
- **Pronto quando:** 3 telas reais do rustploy (serviço, lista de projetos, janela de logs) saem dentro da
  tolerância, e há um documento listando as diferenças aceitas e as recusadas. **Decisão de go/no-go do Taffy.**

#### Fase A2 — Layout próprio em produção

- Trocar, nó a nó (`container`, `row`, `column`, `stack`, depois `grid`, `scrollable` interno), o que
  `render_node` gera. Cada troca atrás de uma chave interna (`GLACIER_LAYOUT=legacy|flex`) para comparar lado a
  lado e voltar atrás sem recompilar.
- Reaproveitar `grid.rs`, `anchored.rs`, `overflow.rs`, `reveal.rs` como casos de teste: se o `GlacierFlex`
  consegue substituí-los, a abstração está certa.
- Fechar as limitações do GSS que dependiam do layout: `max-size` pleno, `fill-portion` correto, centralização
  sem hack, `wrap`. Cada uma ganha teste e entrada no CHANGELOG.
- **Pronto quando:** todos os snapshots da A0 passam com `flex` ligado, o app roda uma semana de uso real sem
  regressão visual relatada, e a chave `legacy` pode ser removida.
- **Risco alto** porque toca todas as telas. Mitigação: a chave de volta, os snapshots, e entrar tela a tela.

#### Fase A3 — Extrair o `glacier-core`

- Criar o workspace do glacier-ui com dois crates: `glacier-core` (parser, eval, estilo, scripts, contexto,
  layout, `EngineMessage`) e `glacier-ui` (fachada que reexporta tudo e traz o backend iced por padrão). Quem
  depende de `glacier-ui` não percebe.
- Substituir os tipos do iced usados fora da fronteira (as categorias do inventário da A0) por tipos
  próprios com `From`/`Into` no adaptador. Meta: `grep iced glacier-core` vazio (a compilação garante, como no
  daemon: se o `core` não depende do `iced` no `Cargo.toml`, um `use` proibido é erro).
- Mover, sem mudar lógica: `parser.rs`, `eval.rs`, `stylesheet.rs`, `luau/`, `forms.rs`, `component.rs`…
- **Pronto quando:** `cargo build -p glacier-core` não compila nem baixa o iced; o `rustploy-gui` compila contra a
  fachada sem mudar uma linha.

#### Fase A4 — A IR e o `trait Backend`

- Definir a IR (`View`): caixa com estilo e filhos, texto, imagem, svg, canvas (já existe `shapes.rs`), campo de
  edição (como nó "folha opaca" que o backend implementa), região rolável.
- Definir o `Backend`: `measure_text`, `paint(&DisplayList)`, `poll_events`, `clipboard`, `cursor`, `window`.
  Princípio: **o menor conjunto que o iced consegue cumprir hoje**; cada método extra tem que ter um uso real.
- `glacier-iced` implementa: IR → árvore de `Element` (a maior parte do `render_node` atual muda de casa).
- **Pronto quando:** `render_node` desaparece do core; o app continua idêntico (snapshots); o adaptador tem
  testes próprios.

#### Fase A5 — Eventos normalizados e drag-and-drop

- Eventos nossos: `Pointer{down,up,move,wheel}`, `Key`, `Focus`, `Ime`, `Window`. O adaptador converte.
- Implementar no core o **hit-testing** e a **captura de ponteiro** (necessários para arrastar). Com isso o
  `onReorder`/`reorderKey` deixam de reescrever o JSON do contexto: o core guarda o estado do arraste.
- Foco e navegação por teclado (Tab/Shift+Tab) com ordem definida pelo layout, não pelo iced.
- **Pronto quando:** o drag-and-drop de lista, de painéis (`panes`/`grip`) e de `Splitter` usa o mecanismo
  único (hoje são três implementações do mesmo gesto, segundo o `PLANO_WIDGETS.md`).

#### Fase A6 — Os widgets próprios sobre a IR

- Reescrever os 7 widgets `advanced` como nós da IR + lógica de evento no core; o desenho vira comandos da
  `DisplayList`. Ordem sugerida, do mais simples ao mais cheio de estado: `spinner`, `animated_toggler`,
  `reveal`, `overflow`, `anchored`, `smooth_scroll`, `grid`.
- `textarea`, `charts`, `gauges`, `canvas` seguem a mesma linha, caso a caso.
- **Pronto quando:** nenhum `impl Widget<…>` do iced fora de `glacier-iced`.

#### Fase A7 — Fechamento

- Publicar (ordem: `glacier-core` → `glacier-iced` → `glacier-ui`), subir no `rustploy-gui`, atualizar
  `AGENTS.md`, `docs/indice` e o `README` do glacier-ui.
- Rever o `vendor/iced_tiny_skia` do `rustploy-gui`: se o renderer ficou atrás do `Backend`, o patch passa a ser
  problema só do adaptador.
- **Pronto quando:** `rustploy-gui` está na versão nova, publicado, sem regressão nas medições da seção 8.

### 4.5 O que o A **não** resolve (para ninguém se enganar)

- **Texto continua sendo do iced** (que usa o `cosmic-text`). O A não melhora shaping, IME nem a11y; só abre a
  porta para trocar isso depois (Caminho B).
- **Acessibilidade continua zero.** Sem a IR e o foco próprios (A4/A5) ela nem seria possível; com eles, ligar um
  AccessKit no adaptador vira trabalho de adaptador (ver 5.4).
- Android e Web continuam dependendo do iced (e do fork do `iced_winit` no Android) até o adaptador próprio deles.

## 5. Caminho B — trocar a camada de baixo por uma que já traga o difícil

O B só começa de verdade **depois da A4**, quando o `trait Backend` existe: aí cada candidato é *um adaptador*.
Antes disso dá para fazer os **spikes** (5.3) em paralelo, porque eles medem o candidato, não o glacier-ui inteiro.

### 5.1 O que o backend precisa entregar

| Capacidade | Por que importa para o rustploy |
|---|---|
| Texto de qualidade (shaping, fallback de fonte, bidi, quebra, seleção) | logs, nomes de serviço, o editor de compose/YAML, texto em pt/en |
| IME e teclas mortas | digitar acentos e CJK em campos de formulário |
| Acessibilidade (árvore semântica para leitores de tela) | requisito legítimo de qualquer ferramenta séria; hoje inexistente |
| Renderizador com **fallback por software** | VM, RDP, hardware sem Vulkan |
| Janelas múltiplas, transparência opcional, DPI fracionário, Wayland/X11/Windows | o app abre janelas de logs/jobs separadas |
| Tamanho do binário e RAM em idle | o projeto vende leveza (o daemon é < 50 MB) |
| Licença compatível (MIT/Apache) | repositório MIT |

### 5.2 Candidatos e hipóteses (a confirmar nos spikes)

| Candidato | O que já traria | Dúvidas que o spike precisa responder |
|---|---|---|
| **Vello + Parley (+ winit + AccessKit)**, via Xilem/Masonry ou direto | renderizador 2D em compute (GPU), texto com Parley, a11y com AccessKit | tem caminho de software? maturidade dos widgets de formulário? custo de manter a cola |
| **Slint** | toolkit completo, a11y, renderer de software próprio, linguagem declarativa | a linguagem declarativa dele *concorre* com `.gv`/`.gss`: dá para usar só como renderizador? licença (GPL/royalty-free/comercial: **conferir**) |
| **egui** | simples, rápido de integrar, renderer wgpu e glow | modo imediato (reconstrói a árvore todo quadro) combina mal com uma IR retida; IME/a11y/estilo são mais fracos |
| **Floem / Dioxus-native (Blitz)** | CSS de verdade (Stylo + Taffy) e Vello no segundo | são jovens; acoplar o GSS ao CSS deles é mais um mundo |
| **Manter iced** | o que já funciona | atrito com o modelo do iced — que o A reduz, mas não zera |

Observação de desenho: um candidato que traz **CSS completo** (Blitz) *substituiria* o GSS em vez de hospedá-lo — é
outro projeto. O B aqui trata de trocar a camada de **desenho e entrada**, não a linguagem de estilo.

### 5.3 Spikes (Fase B1) — um por candidato, todos medidos igual

Cada spike reproduz uma **fatia vertical** do rustploy, sempre a mesma:

1. Tela do serviço com abas, formulário (campos de texto, `select`, `checkbox`, `toggler`), tabela e cartões.
2. Visualizador de logs com 50 000 linhas rolando e seleção de texto.
3. Modal e janela filha (janela de logs).
4. Lista com drag-and-drop de reordenação.
5. Tema claro e escuro, DPI 100% e 150%.
6. Campo de texto com acento (`ã`, `ç`, tecla morta) e, se possível, IME.

Medições (a mesma planilha para todos, na máquina de desenvolvimento **e** numa VM sem GPU):
RAM idle e após 10 min de uso; tamanho do binário release; tempo até a primeira janela; quadros/s e CPU rolando
os logs; linhas de código do adaptador; comportamento sem GPU; Wayland, X11 e Windows; IME; leitor de tela
(Orca/NVDA) lendo o formulário; build de release em Linux e Windows (incluindo o cross-compile com `cargo-xwin`).

### 5.4 Critério de decisão (go/no-go)

Escolher o backend alternativo **só se** ganhar nas três coisas que o iced não dá e perder em nada eliminatório:

- **Elimina:** não abre sem GPU; licença incompatível; exige reescrever a linguagem `.gv`/`.gss`; RAM idle
  muito pior que a linha de base; não roda no Windows com o cross-compile atual.
- **Precisa ganhar:** IME correto, acessibilidade funcional, e texto sem os problemas conhecidos de hoje.
- **Desempata:** menos código de adaptador, binário menor, melhor DPI fracionário.

Se nenhum ganhar, **a decisão é ficar no iced** — e isso é um bom resultado: o A já terá feito o glacier-ui deixar
de ser refém dele.

### 5.5 Fases

| Fase | O que | Esforço | Risco |
|---|---|---|---|
| B1 | Spikes (5.3) e relatório de decisão | M por candidato | baixo (código descartável) |
| B2 | Adaptador do vencedor atrás da feature `backend-<nome>`; iced segue padrão | G | médio |
| B3 | Paridade: todos os snapshots da A0 e os testes do `rustploy-gui` passam no novo backend | G | alto |
| B4 | Virar padrão no desktop; iced fica como opção (Android/Web seguem nele até haver adaptador) | P | médio |

O B2 só inicia com o relatório do B1 aprovado por você (seção 9).

## 6. O que fazer com Android, Web, webview, bandeja e MicroPython

O glacier-ui não é só desktop. Regra: **nenhuma dessas capacidades pode regredir por causa do A ou do B.**

- **Android** (`feature android`, fork do `iced_winit`) e **Web/wasm** (`web-gpu`): continuam no backend iced. O
  adaptador próprio, se vier, é um trabalho à parte; o plano **não** promete outro backend para eles.
- **`webview`** (wry) e **`tray`**: ficam fora da IR (são integrações de SO, não de desenho). Já são módulos
  isolados (`webview.rs`, `tray.rs`, `single_instance.rs`).
- **MicroPython/Luau:** vivem no core; o A só os isola do iced.
- **`daemon.rs`** (multi-janela, 46 refs ao iced): é runtime, não widget. Entra no `Backend` como `window`.

## 7. Compatibilidade, versões e publicação

- O glacier-ui está em 0.122; a fachada mantém o nome e a API. As fases A0 a A2 não mudam API pública (só
  comportamento corrigido). A3 muda a estrutura interna (dois crates) sem mudar a API.
- **Ordem de publicação** (cada passo pronto antes do próximo, com `--dry-run`): `glacier-core` → `glacier-iced` →
  `glacier-ui`; depois `rustploy-gui` sobe a versão. O `rustploy-gui` **nunca** usa `path` para eles.
- Cada fase fecha com um bump do `glacier-ui` e uma entrada no `CHANGELOG.md` dele, e o `rustploy-gui` só sobe
  depois de passar `cargo test --bins` e `--test templates_render` e a revisão visual (receita Xvfb/Xephyr do
  projeto para tela e para teclado).
- Se o `iced` 0.15 sair no meio do caminho, a migração dele **não** é feita antes da A3: depois da fronteira, a
  migração fica restrita ao `glacier-iced`.

## 8. Linha de base e métricas de sucesso

Medir **antes** (Fase A0) e depois de cada fase relevante. Na máquina de desenvolvimento e numa VM sem GPU.

| Métrica | Como medir | Meta |
|---|---|---|
| RAM idle (RSS) após abrir e navegar 5 telas | `/proc/<pid>/status` | não piorar > 5% |
| Tamanho do binário release (Linux e `.exe`) | `ls -l`, `cargo bloat` | não piorar; ideal cair com a saída de dependências |
| Tempo até a primeira janela | log com timestamps | não piorar |
| Tempo de layout das telas grandes | `perf.rs` do glacier-ui | cair ou empatar |
| CPU rolando 50 000 linhas de log | `top`/`perf` | cair ou empatar |
| Snapshots de layout (A0) | `cargo test` | 100% dentro da tolerância |
| Snapshots de imagem (A0) | `cargo test` | diferença de pixels < limite combinado |
| Limitações do GSS fechadas | plano das limitações do GSS (glacier-ui) | contagem, por fase |
| Linhas de código que citam `iced` fora do adaptador | `grep` | tende a zero até o fim da A6 |

## 9. Riscos e cuidados

- **Layout é onde o plano pode morrer.** Diferenças sutis de `Shrink`/`Fill`/`min-content` aparecem só em telas
  reais. Mitigação: snapshots da A0, chave `legacy|flex`, entrar tela a tela, e o go/no-go do Taffy na A1.
- **Texto e foco dentro do layout próprio.** Medir texto exige o backend; um erro de medição desalinha tudo. Os
  campos de edição ficam como *ilhas* do iced até a decisão do B (por isso a A não promete texto próprio).
- **Escopo do glacier-ui crescer.** É um toolkit com ambição "Qt em Rust" (95% do catálogo previsto). O plano
  protege o que já existe; não cria widget novo. Qualquer widget novo durante o A entra na IR, não no iced.
- **Dois mantenedores de uma mesma coisa.** Enquanto `legacy` e `flex` coexistem, todo fix de widget é duplo. Por
  isso a chave tem prazo (removida ao fim da A2).
- **Dependência de terceiros jovens** (Taffy é maduro; Vello/Parley/Xilem/Blitz ainda se movem rápido). Mitigação:
  versões fixas, o adaptador isola, e o B só adota depois do spike.
- **Conflito com trabalho em paralelo no glacier-ui** (o histórico mostra releases quase diárias). Fases curtas,
  integração rápida, e as fases A0/A1 não tocam a API pública.

## 10. Decisões em aberto (para você)

1. **Começar pelo A** (recomendado) ou pular direto para spikes do B? Recomendo A0 e A1 primeiro, porque
   respondem se o Taffy serve e dão a linha de base que o B também precisa.
2. **Taffy** como motor de layout, ou um flex próprio mínimo? Sugestão: Taffy, com contingência na A1.
3. **Dois crates** (`glacier-core` + `glacier-ui`) ou um crate com módulos e `cfg`? Sugestão: dois crates, porque é
   o único jeito de o compilador **garantir** que o core não depende do iced (a mesma lição do daemon).
4. **Android e Web entram no escopo do B?** Sugestão: não; seguem no iced.
5. **Slint está no páreo?** A linguagem declarativa dele concorre com o `.gv`; se a licença e o uso "só como
   renderizador" não passarem, sai da lista antes do spike.
6. **Quando?** Sem pressão do rustploy: o `rustploy-gui` não precisa de nada disto para funcionar. É investimento
   no glacier-ui; convém fazer depois de fechar a migração Podman/dados reais do `TODO.md`.

## 11. Primeiros passos (Fase A0, o que fazer na prática ao aprovar)

1. Rodar e registrar a linha de base (seção 8) no `rustploy-gui` 0.1.8 + `glacier-ui` 0.122.4.
2. No glacier-ui, gerar a tabela de acoplamento (uma linha por uso do `iced`) e commitar em `docs/`.
3. Escrever o *dump de layout* (JSON por tela, viewport fixo) e o teste de comparação; gerar os gabaritos das
   telas do rustploy.
4. Escrever o teste de imagem headless com `tiny-skia` e gerar os PNG de referência.
5. Abrir o spike do `GlacierFlex` (A1) numa tela só e comparar com o gabarito.
