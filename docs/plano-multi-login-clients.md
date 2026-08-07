# Plano: login multi-servidor no client iced + login simplificado na webui

## O problema, em linguagem simples

Hoje os dois clients (`rustploy-gui`, o app iced, e a webui servida pelo
próprio daemon) têm uma tela de login que guarda **um único** par
servidor+token, com dois checkboxes ("Remember server"/"Remember token").
Isso incomoda de duas formas diferentes, uma em cada client:

- **No client iced**, quem administra mais de um daemon rustploy (por
  exemplo, um servidor de staging e um de produção) precisa apagar e
  redigitar URL e token toda vez que troca de servidor. Não existe lista —
  só um par, sobrescrito a cada "remember".
- **Na webui**, pedir a URL do servidor não faz sentido: a webui **é servida
  pelo próprio daemon** (`crates/daemon` serve os arquivos estáticos de
  `webui/`). O servidor a quem falar já é conhecido pelo browser — é a
  própria origem de onde a página veio. Perguntar isso de novo é redundante
  e é campo a mais pra preencher à toa.

Este documento cobre as duas mudanças. São independentes uma da outra (dá
pra fazer só uma), mas o pedido original junta as duas porque nasceram da
mesma frustração com a tela de login.

## Parte 1 — client iced: lista de servidores + dropdown digitável

### Por que isso não é trivial

O glacier-ui (a lib de UI do client iced) hoje só tem um `<Select>`, que
virou um `pick_list` do iced — um dropdown de opções fixas, sem digitação.
Um "dropdown digitável" (digitar E filtrar, ou digitar um valor novo que não
está na lista) é outra coisa: no catálogo de widgets do glacier-ui
(`~/Development/rust/glacier-ui/PLANO_WIDGETS.md`, linha ~90) esse widget já
está mapeado como `ComboEdit` (base `combo_box` do iced), mas está **só no
planejamento** — não tem implementação ainda (status 🟡/⬜).

Por isso a primeira etapa deste plano não é no rustploy, é no glacier-ui.

Tem também uma diferença arquitetural importante entre os dois:
`pick_list` é **sem estado** (o motor recria a lista de opções do zero a
cada frame, a partir de uma string JSON no contexto — é assim que `Select`
já funciona, ver `widget.rs:849`). Já o `combo_box` do iced é **com
estado**: ele precisa de um `combo_box::State<String>` que sobrevive entre
frames (perder esse estado a cada repaint reseta o campo de busca digitado).

O glacier-ui já resolve esse tipo de problema para outro widget parecido: o
`TextEditor` multi-linha mantém um mapa `HashMap<String, text_editor::Content>`
vivo no motor, indexado pelo nome do binding do contexto (`EditorMap` em
`widget.rs:63`, populado em `lib.rs:1063`/`1800`). O `ComboEdit` vai seguir o
mesmo padrão: um `HashMap<String, combo_box::State<String>>` keyed pelo
binding de `options`, criado na primeira vez que o node aparece e atualizado
(`state.update(&novas_opcoes)`) só quando a lista de opções realmente muda.

### O que entra no glacier-ui

1. `NodeType::ComboEdit` em `parser.rs` (mesmos atributos de `Select`:
   `options`, `value`, `on_change`/`on_select`, `placeholder`, mais o texto
   livre digitado que não precisa bater com nenhuma opção).
2. Um braço novo em `widget.rs` usando `iced::widget::combo_box`, com o mapa
   de estado descrito acima.
3. Dois eventos possíveis de disparar pro Luau: "texto digitado mudou" (a
   cada tecla, pra permitir digitar um servidor novo) e "item da lista
   selecionado" (quando o usuário clica ou confirma um item existente — é
   esse que vai disparar o autopreenchimento do token no rustploy-gui).
4. Exemplo em `examples/` (convenção do projeto: rodar o exemplo antes de
   considerar pronto, não só `cargo test`).
5. Bump de versão (`0.56.0` → `0.57.0`), `cargo publish --dry-run` e depois
   `cargo publish`.

### O que entra no rustploy-gui depois

1. Subir a dependência `glacier-ui` em `crates/rustploy-gui/Cargo.toml` pra
   `0.57`, `cargo check -p rustploy-gui`.
2. **Armazenamento**: hoje `handlers/connection.luau` guarda um único par em
   `storage` (`prefs.url`/`prefs.token`, ver `connection.luau:18-27` e
   `:98-114`). Isso vira uso uma **lista** de pares:
   ```lua
   storage.set("saved_servers", {
       { url = "https://prod.example.com", token = "..." },
       { url = "https://staging.example.com", token = "..." },
   })
   ```
3. **Tela (`login.gv`)**: o `<input form_control="url">` vira um
   `<ComboEdit>` ligado a `ctx.url`, com `options` = lista de URLs salvas.
   Ao **selecionar** um item da lista (não a cada tecla), o handler busca o
   token correspondente em `saved_servers` e preenche `ctx.token`
   automaticamente. Se o texto digitado não bate com nenhum item salvo, o
   campo continua editável normalmente — é assim que se cadastra um
   servidor novo.
4. **Salvar automaticamente** (decisão já tomada): toda conexão
   bem-sucedida (`DaemonStatus` responde OK) salva/atualiza o par na lista —
   sem precisar de checkbox "remember". Os dois checkboxes atuais
   ("Remember server"/"Remember token") saem da tela.
5. **Remover um servidor salvo**: like a lista cresce e não tem mais
   checkbox pra "esquecer", precisa de algum jeito de apagar uma entrada —
   proposta: um "×" pequeno ao lado de cada item quando a lista aparece
   aberta (ou uma listinha simples abaixo do combobox com botão de remover
   cada linha). Vou decidir o layout exato na hora de implementar a tela,
   mas o comportamento (remover do `storage`) já fica registrado aqui.

## Parte 2 — webui: só pedir o token

### Mudança

- `app.js`: `connect()` para de usar `this.url`/`normalizeUrl()` e passa a
  montar o `Api` direto com `window.location.origin`. Os campos `url`,
  `rememberUrl`, `erroUrl` saem do store.
- `persistPrefs()` (e o `localStorage` `rustploy.prefs`) guardam só
  `{ rememberToken, token }`.
- `index.html`: sai o bloco "SERVER URL" (linhas 40-48) e o checkbox
  "Remember server" (linhas 60-63) da tela de login. Sobra só o campo de
  token e o checkbox "Remember token".
- `screens/login.js` não muda (só chama `store.connect()`).

### Por que isso é seguro

`net/api.js` já monta a URL de request só como `baseUrl + "/api/rpc"` — não
depende de path prefix nenhum hoje. `window.location.origin` dá
exatamente `scheme://host:porta` de onde a página HTML foi carregada, que é
sempre o daemon que serve a própria webui. Não há caso hoje em que a webui
é servida de um host e fala com a API de outro.

## Ordem de execução

1. glacier-ui: implementar e publicar `ComboEdit`.
2. rustploy-gui: subir dependência, `cargo check`.
3. rustploy-gui: `saved_servers` no storage + tela de login nova.
4. webui: simplificar login pra só token.
5. Testar de verdade os dois clients (`cargo run -p rustploy-gui` e abrir a
   webui no browser) antes de dar por pronto — checklist do projeto.

## Escopo às claras

Isso é trabalho em **dois repositórios** (`glacier-ui` e `rustploy`) e a
parte 1 é bem maior que a parte 2 por causa do widget novo. Registro aqui
pra não subestimar: o combobox de verdade é o item mais caro do pedido.
