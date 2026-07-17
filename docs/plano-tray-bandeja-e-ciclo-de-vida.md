# Plano — ícone de bandeja e o app que sobrevive à última janela

> Status: **IMPLEMENTADO** (2026-07-17). glacier-ui **0.47.0** publicada com a
> feature `tray`; rustploy-gui subiu a dep e ligou `.tray()`/`.on_tray()`.
>
> **Onde ficou cada peça:**
> - glacier: `src/tray.rs` (bandeja + thread/loop por plataforma + gate de
>   notificações), `src/daemon.rs` (`.tray()`/`.on_tray()`, sobreviver à última
>   janela, reabrir a principal), `examples/bandeja`. Feature `tray` (opt-in).
> - rustploy-gui: `src/app/mod.rs` (`tray_config()` + `handle_tray()`),
>   `Cargo.toml` (dep `features=["tray"]` + `Depends` do `.deb`).
>
> **Falta validar manualmente** (não dá para automatizar aqui): clicar nos 3
> itens do menu, conferir que fechar a janela recolhe para a bandeja e que "Open
> Rustploy" reabre. O boot da bandeja já foi verificado rodando o app (sem panic,
> GTK inicializou). No **Windows** o caminho (`run_loop` Win32) não foi compilado
> nesta máquina Linux — validar num build Windows.

---

## O que o usuário pediu

Hoje o rustploy-gui abre, mostra a janela e, quando a última janela fecha, o app
inteiro morre. O pedido é mudar esse ciclo de vida:

1. **Ao abrir**, criar um **ícone na bandeja do sistema** (system tray / área de
   notificação).
2. **Ao fechar a última janela**, o app **não encerra** — ele continua vivo em
   segundo plano, representado só pelo ícone da bandeja.
3. O ícone da bandeja tem um **menu com 3 itens**:
   - **Disable / Enable notifications** — liga/desliga as notificações nativas do
     SO (as que já existem hoje no desfecho de um deploy).
   - **Open Rustploy** — reabre (ou traz para a frente) a janela principal.
   - **Quit Rustploy** — aí sim encerra o app de verdade.

## Por que isto encosta na glacier-ui (e não é só código do rustploy)

Quem é dono do loop de eventos do app é a **glacier-ui**, não o rustploy. Desde a
0.38 o `app/mod.rs` do rustploy é **só configuração**: ele monta um
`GlacierDaemon` (fontes, janela borderless, ganchos de persistência) e chama
`.run()`. Dentro desse `.run()` mora o loop do `iced::daemon` — abrir janelas,
rotear cliques, e **decidir encerrar o app**.

E é exatamente aí que está a trava. No runner da glacier
(`glacier-ui/src/daemon.rs`), quando a última janela some:

```rust
DaemonMessage::Closed(id) => {
    self.windows.remove(&id);
    ...
    if self.windows.is_empty() {
        iced::exit()          // ← o app morre aqui
    } else {
        Task::none()
    }
}
```

Três das quatro coisas que o usuário pediu vivem desse lado da fronteira:

- **Não morrer na última janela** → mudar essa decisão de `iced::exit()`.
- **Reabrir a janela principal** ("Open Rustploy") → só o runner sabe recriar um
  motor e abrir uma janela; ele hoje até **joga fora** o `setup`/settings da
  principal depois do boot, então nem teria como reabrir.
- **O ícone da bandeja em si** → o menu precisa injetar eventos no loop do
  `iced`/`winit`, e esse loop é da glacier.

A convenção do projeto (registrada no cabeçalho do `app/mod.rs` e no CLAUDE.md) é
clara: *"se algo aqui parecer faltar, o lugar de consertar é o builder do glacier,
não um runtime paralelo aqui."* Já houve um runtime `iced::daemon` reimplementado
à mão no rustploy que foi removido justamente por isso. Então a bandeja entra
como **recurso do `GlacierDaemon`**, e o rustploy só a **declara**.

A quarta coisa (ligar/desligar notificações) também é da glacier: é ela quem tem o
`notify()` e quem de fato dispara a notificação do SO (`emit_os_notification` em
`lib.rs`). O toggle vira um interruptor global dentro da glacier que o `notify()`
consulta antes de emitir.

## Desenho proposto

### Lado glacier-ui (nova versão — provável 0.47.0)

Um bump de *minor* porque muda comportamento (a última janela deixa de encerrar
quando há bandeja) e adiciona API. Peças:

**1. Configuração da bandeja no builder**

```rust
GlacierDaemon::new()
    // …config atual…
    .tray(TrayConfig {
        icon: RUSTPLOY_PNG_BYTES,     // mesmos bytes do ícone da janela
        tooltip: "Rustploy",
        items: vec![
            TrayItem::check("notifications", "Disable notifications"), // marcável
            TrayItem::separator(),
            TrayItem::button("open",  "Open Rustploy"),
            TrayItem::button("quit",  "Quit Rustploy"),
        ],
    })
    .on_tray(|item_id: &str, tray: &mut TrayActions| match item_id {
        "open"          => tray.open_main(),
        "quit"          => tray.quit(),
        "notifications" => tray.toggle_notifications(),
        _ => {}
    })
```

`TrayActions` (entregue ao gancho) oferece as ações que dependem do runner:
`open_main()`, `quit()`, `toggle_notifications()` e utilitários de menu
(`set_checked(id, bool)`, `set_label(id, text)`) — para o item alternar entre
"Disable"/"Enable" e mostrar o check.

**2. Ciclo de vida: sobreviver à última janela**

Com uma bandeja configurada, a decisão do `Closed` passa a ser:

```rust
if self.windows.is_empty() && self.tray.is_none() {
    iced::exit()          // sem bandeja: comportamento de hoje, intacto
} else {
    Task::none()          // com bandeja: fica vivo em segundo plano
}
```

O loop do `iced::daemon` continua rodando com **zero janelas** porque as
subscriptions (a da bandeja, os ticks) o mantêm acordado; `view` só é chamado por
janela, então zero janela = loop ocioso, que é o que queremos. Só o **Quit**
(`iced::exit()`) encerra de fato.

**3. Reabrir a janela principal ("Open Rustploy")**

Hoje o `boot` consome `setup`, `main_settings` e `main_title` e os descarta. Para
reabrir, o `Runtime` passa a **guardá-los** (`setup` como `Rc`, settings/título
clonados). `open_main()` então:

- se já existe janela principal viva → **traz para a frente**
  (`window::gain_focus(main_id)`);
- se não → recria um motor com o `setup`, faz `window::open(main_settings)`,
  atualiza o `main_id`. Fica idêntica à do boot (mesma geometria salva, mesmo
  chrome), reaproveitando a geometria persistida pelo `on_close`.

**4. Toggle de notificações**

Um `AtomicBool` global na glacier (default: ligado). O disparo em `lib.rs` passa a
consultá-lo:

```rust
for spec in notifications {
    if notifications_enabled() {
        std::thread::spawn(move || emit_os_notification(spec));
    }
}
```

`TrayActions::toggle_notifications()` inverte o átomo e atualiza o rótulo/checkbox
do item ("Disable notifications" ↔ "Enable notifications"). Como é estado global
de processo, não precisa passar pela camada Luau — o `notify()` do
`stream.luau` continua igual, só que respeitado ou ignorado conforme o toggle.

**5. Integração com o SO (a parte espinhosa) — crate `tray-icon`**

`tray-icon` (do ecossistema Tauri) é o padrão cross-platform. Os cliques do menu
chegam por um canal global (`MenuEvent::receiver()`), que uma subscription do
`iced` drena e converte em `DaemonMessage::Tray(item_id)` (ponte canal→stream,
mesmo padrão dos streams SSE já existentes).

O detalhe por plataforma:

- **Linux** (alvo primário, empacotado `.deb`): `tray-icon` usa GTK e precisa de
  um **loop GTK vivo**. O `iced`/`winit` não roda GTK, então a bandeja sobe numa
  **thread dedicada** que faz `gtk::init()`, cria o ícone e roda `gtk::main()`.
  Os eventos de menu continuam vindo pelo canal global — a thread só serve o loop
  GTK. Requer as libs `libgtk-3` / `libayatana-appindicator3` (adicionar às
  `Depends` do `.deb`).
- **Windows** (empacotado `.zip`/`.exe`): mesma thread dedicada, com o bombeamento
  de mensagens Win32 que o `tray-icon` gerencia.
- **macOS**: a bandeja **exige a thread principal** — a thread-dedicada não
  funciona. Como o macOS **não** está nos alvos de empacotamento hoje, fica como
  **limitação conhecida** (a bandeja simplesmente não sobe; o app volta a
  encerrar na última janela via o `tray.is_none()`). Tratar depois, se e quando
  o macOS entrar.

### Lado rustploy-gui (mínimo)

Depois de publicar a glacier 0.47.0 e subir a dep (fluxo obrigatório do
CLAUDE.md — nunca `path`/`[patch]`):

- `app/mod.rs`: acrescentar `.tray(...)` + `.on_tray(...)` na construção do
  `GlacierDaemon`, reusando os bytes do ícone que já são embutidos para a janela.
  O gancho mapeia os 3 itens para `open_main`/`quit`/`toggle_notifications`.
- `Cargo.toml` (`[package.metadata.deb]`): adicionar as `Depends` de GTK/app-indicator.
- Documentar aqui o desfecho (mover este arquivo de "plano" para "relatório").

O `main_window_settings`, a persistência de geometria e todo o resto do chrome
**não mudam** — reabrir a principal reaproveita o mesmo caminho.

## Decisões (confirmadas)

- **Fechar no X = recolher para a bandeja.** Com a bandeja, clicar no X da janela
  não encerra mais o app — ele "recolhe" para a bandeja. (O `on_close` continua
  salvando a geometria antes.)
- **Interação com o ícone**: **clique esquerdo** no ícone → `open_main()`
  (reabre/foca a janela); **clique direito** → mostra o menu de 3 itens.
  Ressalva Linux (SNI/appindicator): alguns ambientes só entregam o **menu**
  (em qualquer clique) e não um evento de clique-esquerdo separado — nesses, o
  item "Open Rustploy" do menu cobre a reabertura. No Windows o clique-esquerdo
  funciona normal.
- **Notificações começam ligadas.**

## Plataformas-alvo da bandeja (confirmado: Linux + Windows)

Mira **Linux + Windows** agora. macOS fica como **limitação conhecida** (a bandeja
exige a thread principal; sem ela, `tray.is_none()` faz o app voltar a encerrar na
última janela). Tratar quando/se o macOS entrar no empacotamento.

## Passos de execução (resumo)

1. glacier-ui: dep `tray-icon`; `TrayConfig`/`TrayItem`/`TrayActions`; guardar
   `setup`/settings no `Runtime`; subscription da bandeja; gate de notificações;
   ajustar o `Closed`.
2. glacier-ui: exemplo + teste; `cargo publish --dry-run`; bump 0.46→0.47;
   `CHANGELOG`; publicar.
3. rustploy-gui: subir a dep; `.tray()`/`.on_tray()` no `app/mod.rs`; `Depends`
   no `.deb`; `cargo check -p rustploy-gui`.
4. Rodar de verdade (não só teste): abrir, fechar a janela, conferir que fica na
   bandeja, os 3 itens, reabrir, quit. (Ver skill `run`.)
