# Plano: webui com acabamento da GUI, janelas arrastáveis e paridade total

> Status: **implementado** (2026-10-07). Cada item é marcado `[x]` quando
> cumprido; o que mudar de ideia no meio do caminho fica anotado em "Notas".

## Por que

A webui nasceu como tradução 1:1 das telas da GUI (`rustploy-gui`, glacier-ui).
Hoje ela cobre quase tudo, mas tem três defeitos:

1. **Cara de protótipo.** Tokens e classes copiados, porém sem o acabamento que
   a GUI tem: sem ícones de verdade (glifos unicode soltos), sem foco/hover
   consistentes, sem profundidade (sombras), sem animação de entrada, vazios
   sem personalidade, barras de rolagem do navegador, tabelas chapadas.
2. **Criar coisas “sai” da tela.** Na GUI, *Novo projeto*, *Novo serviço*,
   *Novo job*, *Novo token* e *Logs* são **janelas** separadas. Na webui, uns
   são formulário inline, o wizard de serviço troca a tela inteira, e os
   “modais” travam a página com um fundo escuro e não se movem.
3. **Lacunas de funcionalidade** (ver inventário abaixo).

## Decisão de arquitetura: um gerenciador de janelas pequeno

Não vale reescrever 2 500 linhas de HTML. Os “modais” atuais já têm uma
estrutura comum (`modal_backdrop > modal_box > modal_head + modal_body`).
Em vez de trocar cada um, **a mesma estrutura vira janela** por CSS + uma
diretiva Alpine (`x-win`), e os dois formulários que hoje não são modais
(Novo projeto, Novo serviço) ganham a mesma casca:

- `backdrop` deixa de escurecer/bloquear: vira só a camada onde as janelas
  vivem (`pointer-events: none`); a janela em si recebe cliques. A página
  por baixo continua usável, como na GUI (várias janelas ao mesmo tempo).
- `x-win="chave"` liga: **arrastar** pela barra de título, **redimensionar**
  (alça no canto, nativa `resize: both`), **trazer à frente** ao clicar
  (contador de z-index), **maximizar/restaurar** (duplo clique na barra ou
  botão), **fechar com Esc** (só a de cima), **manter dentro da tela**
  (clamp), e **lembrar posição/tamanho** por chave em `localStorage`.
- Em telas estreitas (≤ 700 px) a janela vira tela cheia (arrastar não faz
  sentido no celular) — mantém o PWA utilizável.
- Respeita `prefers-reduced-motion`.

## Inventário de lacunas funcionais (webui × GUI)

| Item | GUI | webui | Ação |
|---|---|---|---|
| Webhook de deploy (URL, copiar, regenerar) na aba Deployments | sim | **não** | implementar (`GetWebhookUrl`, `RegenerateWebhookToken`) |
| Deploy Engine em 3 abas (Fila / Executando / Histórico 24h) | sim | seções empilhadas | converter para abas com contadores |
| Reordenar a fila arrastando (`DeployQueueReorder`) | sim | só “promover” | arrastar linhas da fila |
| Support | placeholder (só um título) | — | divergência de propósito, não implementar |
| Novo projeto / serviço / job / token / logs em janela | sim | inline/tela/modal fixo | janelas (acima) |
| Editar projeto | inline na GUI | inline | virar janela também (consistência) |

(O restante do inventário de `docs/inventario-paridade-telas.md` continua
valendo; esta lista só traz o que ainda faltava.)

## Polimento visual (o que “acabamento” significa aqui)

Mantém a identidade terminal/mono e a paleta — não é redesenho, é refinamento.

- Tokens novos em `:root`: sombras (`--shadow-sm/md/lg`), raios, `--focus`
  (anel de foco azul), durações/easing de transição.
- **Ícones SVG** (conjunto inline estilo Lucide, sem dependência externa)
  na sidebar, botões de ação, estados vazios e barra de título das janelas.
- **Foco visível** em todo controle (teclado), `:active` com leve pressão,
  `:disabled` consistente.
- **Botões** unificados (primário/ghost/perigo) com transição; versão `sm`.
- **Campos**: foco com anel, placeholder legível, `select`/`textarea`
  estilizados iguais, erro com borda vermelha.
- **Tabelas**: cabeçalho sticky, linha com hover, zebra sutil opcional,
  coluna de ação alinhada.
- **Cards**: hover com elevação e borda de destaque; card de projeto/serviço
  com cabeçalho mais claro.
- **Sidebar/topbar**: indicador lateral do item ativo, agrupamento, marca com
  ícone; topbar com status do daemon como pílula.
- **Barras de rolagem** finas (webkit + `scrollbar-width`).
- **Estados vazios** com ícone + frase + ação sugerida.
- **Toasts** com barra de progresso do tempo de vida.
- **Animações** curtas: entrada de janela/toast, troca de aba (fade),
  skeleton na carga inicial.
- **Responsivo** revisado depois das janelas.

## Fases e checklist

### Fase 0 — Infra de desenvolvimento
- [x] Daemon de dev isolado (config própria, Docker falso por socket, porta
  19797) + servidor que serve `webui/` direto do disco com proxy `/api`
  (sem recompilar o daemon a cada edição). Receita em "Notas".
- [x] Dados de seed (2 projetos, 2 serviços).

### Fase 1 — Gerenciador de janelas
- [x] `wm.js`: diretiva `x-win` (arrastar, redimensionar, foco/z-index,
  maximizar, Esc, clamp, persistência, tela cheia no mobile).
- [x] CSS `.modal_*` → janela (sombra, barra de título com ícone e botões,
  alça de resize, animação).
- [x] Converter os 5 modais existentes: detalhe de deploy, build log, logs
  de job, novo job/editar job, novo token de registry.
- [x] **Novo projeto** em janela (substitui o form inline).
- [x] **Editar projeto** em janela (lista e tela de projeto).
- [x] **Novo serviço** (wizard) em janela (deixa de ser `view`;
  `openNewService()` abre a janela, criação fecha e abre o serviço).
- [x] Logs ao vivo do serviço também destacáveis em janela (como `log_window`
  da GUI) — botão “Abrir em janela” na aba Logs.
- [x] `sw.js` e índice: incluir `wm.js` no app shell.

### Fase 2 — Polimento visual
- [x] Tokens + foco + botões + campos + scrollbars.
- [x] Conjunto de ícones SVG e aplicação (sidebar, ações, vazios).
- [x] Sidebar/topbar refinadas.
- [x] Tabelas e cards (hover/sticky/elevação — revisão fina por tela pendente).
- [x] Estados vazios (ícone automático em `x-fallback`).
- [x] Barra de tempo dos toasts e classe `.skeleton` (shimmer).
- [x] Login com mesmo acabamento.
- [x] Revisão responsiva (janela tela-cheia no mobile, testada via iframe 420px).

### Fase 3 — Paridade funcional
- [x] Webhook de deploy na aba Deployments.
- [x] Deploy Engine em 3 abas.
- [x] Reordenar fila por arrastar (`DeployQueueReorder`).
- [x] Revisão tela a tela contra a GUI (inventário atualizado; Support é a única divergência intencional).

### Fase 4 — Verificação e documentação
- [x] Conferência visual no navegador de cada tela e de cada janela
  (arrastar, redimensionar, empilhar, Esc, maximizar).
- [x] Console do navegador sem erros.
- [x] `cargo test -p rustploy --bins headless_tests` (se Chrome sobe) e
  `cargo check`.
- [x] Atualizar `AGENTS.md` (webui: janelas), `docs/indice` (`make index`) e
  `docs/inventario-paridade-telas.md`.
- [ ] Commit no `rustploy-daemon` e ponteiro no agregador (deixado para o usuário: não foi pedido commit).

## Riscos e armadilhas já conhecidas

- **`flex-shrink: 0`** em filhos de `.scroll_fill`/janelas: toda caixa nova
  dentro de janela com `overflow` precisa disso (ver `AGENTS.md`).
- **Cache imutável** do JS servido pelo daemon: testar sempre pelo servidor
  de dev (sem cache) ou com ctrl+shift+r.
- **Alpine e DOM assíncrono**: o `x-show` aplica em microtask; a diretiva não
  pode medir a janela no mesmo tick em que ela é exibida.
- **`<tag>` literal em comentário HTML** não quebra aqui (diferente do
  glacier), mas `build.rs` remove comentários `<!-- -->` — nada crítico dentro.
- **Teste headless** (`headless_tests`) semeia o store à mão: ao mover o
  wizard de `view` para janela, ajustar o que ele espera.

## Notas de execução

- Dev: `RUSTPLOY_CONFIG=<scratch>/dev/config.toml ./target/debug/rustployd`
  com `[docker] socket_path` apontando para um servidor Unix que responde
  `[]` (script Python) — assim o daemon real/produção nunca é tocado.
  `devserver.py` serve `webui/` do disco em `:19999` e repassa `/api/*`.
