# Correção: a janela não reabria no último tamanho (glacier 0.49.0 → 0.49.1)

> **Nota (2026-08-28, glacier-ui 0.59.0):** o tamanho default citado abaixo
> (1280×820) não vem mais do `main_window_settings()` em Rust — é declarado no
> `<screen size="1280 820">` de `views/app.gv`. A ordem de precedência não
> mudou para o usuário: a geometria salva continua ganhando do default, seja ele
> do builder ou do template. Ver AGENTS.md, "Título e tamanho de janela moram no
> `.gv`".

## Sintoma

Depois de migrar a persistência da geometria da janela para o recurso **nativo**
do glacier-ui (`GlacierDaemon::remember_window_geometry(true)`, glacier 0.49.0) e
remover o antigo `src/app/store.rs`, o app **sempre reabria no tamanho default**
(1280×820). O arquivo `~/.local/share/rustploy/window-geometry.json` **nunca era
criado** — ou seja, a geometria não estava sendo gravada ao fechar.

## Causa

O fluxo de fechamento da janela principal no glacier é:

1. A WM (ou o botão de fechar da titlebar custom) pede para fechar →
   `DaemonMessage::CloseRequested(id)`.
2. `Runtime::close(id)` decide se precisa **consultar** a geometria antes de
   fechar. Se precisar, faz `window::size` + `window::position` e emite
   `DaemonMessage::CloseWithGeometry`, que é onde a geometria é **gravada**.
3. Só então a janela fecha de fato.

O bug estava no passo 2. A condição era:

```rust
// glacier 0.49.0 — src/daemon.rs
fn close(&mut self, id: window::Id) -> Task<DaemonMessage> {
    if id != self.main_id || self.on_close.is_none() {
        return window::close(id);   // fecha DIRETO, sem consultar a geometria
    }
    // ... consulta a geometria e emite CloseWithGeometry (grava aqui)
}
```

A consulta da geometria só acontecia quando havia um gancho **`on_close`**
registrado. Esse gancho era o mecanismo **antigo** (o app fazia a persistência à
mão via `on_close`). Quando o rustploy migrou para a persistência nativa, ele
**removeu** o `.on_close(...)` — então `self.on_close.is_none()` era `true`, o
`close` fechava direto, o `CloseWithGeometry` nunca disparava, e nada era gravado.

Em resumo: **a persistência nativa (`remember_window_geometry`) e a consulta de
geometria ao fechar estavam acopladas ao gancho `on_close`, que a persistência
nativa justamente dispensa.** As duas formas de precisar da geometria não estavam
ambas cobertas.

## Correção (glacier 0.49.1)

O fechamento passou a consultar a geometria quando há um `on_close` **ou** quando
a persistência nativa está ligada (que semeia um `geometry_dir`):

```rust
// glacier 0.49.1 — src/daemon.rs
fn close(&mut self, id: window::Id) -> Task<DaemonMessage> {
    if id != self.main_id || !self.needs_geometry_on_close() {
        return window::close(id);
    }
    // ... consulta a geometria e emite CloseWithGeometry (grava aqui)
}

/// Se o fechamento da principal precisa consultar a geometria antes de fechar:
/// quando há um gancho `on_close` OU a persistência nativa está ligada
/// (`remember_window_geometry`, que semeia o `geometry_dir`).
fn needs_geometry_on_close(&self) -> bool {
    self.on_close.is_some() || self.geometry_dir.is_some()
}
```

O resto do caminho (`window::size` → `CloseWithGeometry` → gravação em
`window-geometry.json`) já existia e não mudou — só o **portão** que decidia se
esse caminho seria tomado é que estava incompleto.

Teste de regressão adicionado no glacier
(`remember_geometry_consulta_geometria_ao_fechar_sem_on_close`): confirma que,
com `geometry_dir` semeado e **sem** `on_close`, `needs_geometry_on_close()`
devolve `true`.

## Impacto no rustploy

Nenhuma mudança de código no rustploy além de subir a dependência
`glacier-ui` de `0.49.0` para `0.49.1` (`crates/rustploy-gui/Cargo.toml`). A
wiring continua a mesma: `.storage_dir(shared::fallback_data_dir())` +
`.remember_window_geometry(true)` em `src/app/mod.rs`.

## Lição

Ao introduzir um caminho **novo** para um comportamento (persistência nativa de
geometria) que antes só existia por um caminho **antigo** (gancho `on_close`),
conferir todos os pontos onde o caminho antigo era o *gatilho* de alguma etapa —
aqui, a decisão de "consultar a geometria antes de fechar" estava condicionada
só à existência do `on_close`, e o caminho novo não a acionava. O bug passou
pelos testes unitários (que cobriam a gravação/leitura isoladas) porque o
*acoplamento* estava no runtime `iced::daemon`, que não é exercitado headless —
só apareceu no uso real (fechar → reabrir). Reforça a regra de sempre validar
uma feature de UI rodando o app de verdade, não só com `cargo test`. Ver o plano
em `docs/plano-file-io-luau-e-geometria.md`.
