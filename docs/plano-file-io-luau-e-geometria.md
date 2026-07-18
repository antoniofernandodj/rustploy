# Plano: I/O de arquivo no Luau + geometria da janela fora do Rust

> **Status: FEITO** (2026-07-18, Opção A). glacier-ui **0.49.0** publicado com
> `fetch("file://…")` (leitura), o global `write_file(path, conteúdo)` (escrita)
> e `GlacierDaemon::remember_window_geometry(true)` (persistência nativa da
> geometria). No rustploy: `crates/rustploy-gui/src/app/store.rs` **removido**,
> `on_close`/`save_geometry`/restauração em `main_window_settings` removidos, e o
> builder agora só liga `.remember_window_geometry(true)`. Não sobrou I/O de
> arquivo local no Rust do `rustploy-gui`.

## Por que este doc existe

Hoje a crate `rustploy-gui` ainda tem um punhado de Rust que é pura "casca":
entrada do binário, localização de assets, config do builder do glacier e
**persistência local em disco** (`src/app/store.rs`). A pergunta que originou
este trabalho foi: *dá pra reduzir ainda mais o Rust, empurrando coisa pra Luau?*

A resposta honesta é que quase tudo que sobrou **tem** que ser Rust — roda antes
do motor Luau existir, ou é API do builder do glacier (fontes embutidas, bandeja,
janela borderless). A única exceção real é o `store.rs`: ele só existe porque a
camada Luau, até agora, **não tinha como ler/gravar arquivo**.

Mas isso é uma limitação removível. O próprio glacier já prova o padrão: o global
`storage` (o "localStorage" do glacier) é I/O de arquivo JSON em disco. Então a
ideia é:

1. **Dar ao Luau I/O de arquivo de verdade** — `fetch("file://…")` pra ler e
   `write_file(path, conteúdo)` pra gravar. Feito na lib `glacier-ui`, é um
   recurso geral (útil pra qualquer app glacier, não só o rustploy).
2. **Mover a persistência da geometria da janela pra fora do `store.rs`**, usando
   essa capacidade nova, pra o `store.rs` deixar de existir.

## Parte 1 — I/O de arquivo no Luau (glacier-ui)

### Leitura: `fetch("file://…")`

O `fetch` já tem um caminho limpo (`prelude.luau` → `parse_fetch` →
`net::send`). Basta um desvio no começo de `net::send` (`src/net.rs`): se a URL
começa com `file://`, em vez de montar a request hyper, ler o arquivo com
`tokio::fs::read_to_string` e devolver o **mesmo formato de sempre**:

```rust
if let Some(path) = req.url.strip_prefix("file://") {
    return Ok(match tokio::fs::read_to_string(path).await {
        Ok(body) => FetchResult { ok: true,  status: 200, body, error: String::new() },
        Err(e)   => FetchResult { ok: false, status: 404, body: String::new(), error: e.to_string() },
    });
}
```

Do lado Luau nada muda na forma: `local res = fetch("file://"..caminho)` devolve
`{ ok, status, body, error }`. É blocante? Não — `tokio::fs` roda no executor
async, igual ao caminho HTTP.

### Escrita: `write_file(path, conteúdo)`

Simétrico à escrita do `storage`. Como escrever em disco local é rápido e não
precisa suspender corrotina, o mais simples é instalar um **global síncrono** em
Rust (igual ao `install_storage`), exposto via prelude:

```lua
local ok, err = write_file(path, conteudo)   -- ok: boolean, err: string?
```

Do lado Rust, um `create_function` que faz `std::fs::write` (criando o diretório
pai se preciso) e devolve `(true)` ou `(false, mensagem)`. Sem pânico: falha de
I/O vira valor de retorno, não erro que derruba o script.

> Observação de segurança: em browser, `fetch("file://")` é bloqueado de
> propósito (código remoto não confiável). Aqui é o oposto — o Luau é código do
> **próprio app**, empacotado junto, tão confiável quanto o Rust. Acesso ao FS a
> partir dele é esperado, não uma brecha.

## Parte 2 — Geometria da janela sem `store.rs`

Aqui está a sutileza que precisa de decisão. A persistência da geometria tem dois
momentos, e eles têm **timings opostos**:

- **Salvar** acontece ao *fechar* — o glacier já entrega a geometria consultada
  na hora ao gancho `on_close(|_, geometry| …)`. Esse momento é alcançável pelo
  Luau sem drama.
- **Restaurar** acontece ao *abrir* — a janela precisa nascer já no tamanho
  certo, o que é **antes de qualquer motor Luau existir**. Nesse instante não há
  Luau rodando pra ler nada.

Ou seja: a **escrita** pode virar Luau; a **leitura pra dimensionar a janela no
boot** é intrinsecamente "casca" (pré-motor). Isso abre duas arquiteturas:

### Opção A (recomendada) — persistência de geometria nativa no glacier

O glacier ganha uma persistência de geometria **opt-in**, backed pelo `storage`
que ele já gerencia:

- **No boot**: antes de criar a janela, o glacier lê a geometria do `storage`
  (uma chave tipo `__window_geometry`) e dimensiona a principal. Sem Luau, **sem
  flash** (a janela já nasce no tamanho certo).
- **No fechar**: o glacier grava a geometria atual no mesmo `storage`.

Efeito no rustploy: **`store.rs` some inteiro** (60 linhas), mais `save_geometry`
e a lógica de restauração em `main_window_settings` (`src/app/mod.rs`). Sobra só
`.remember_window_geometry(true)` (ou implícito quando há `storage_dir`) no
builder. É infra reutilizável por qualquer app glacier.

Custo: a geometria não passa pelas novas primitivas `write_file`/`file://` — usa
o `storage` direto no Rust do glacier. As primitivas da Parte 1 continuam sendo
entregues como recurso geral, só não é a geometria que as exercita.

### Opção B — geometria via Luau (usa `write_file`/`file://`)

Mais alinhada ao lema "toda a lógica vive em Luau", mas com uma costura a mais:

- **Salvar**: o glacier, no fechar, chama um handler Luau passando a geometria; o
  Luau faz `write_file(caminho, json)`.
- **Restaurar**: como o boot é pré-motor, ou (b1) o glacier lê esse mesmo arquivo
  pra dimensionar (acoplamento por convenção de caminho), ou (b2) a janela abre
  no default e o Luau, no `init()`, lê via `fetch("file://")` e chama uma ação
  nova `window:resize` — o que causa um **flash** visível (abre num tamanho,
  salta pro salvo). No Wayland a posição não é restaurável de qualquer forma
  (já documentado), então só o tamanho está em jogo.

O `store.rs` também some, mas trocamos 60 linhas de Rust limpo por uma costura de
ciclo de vida mais frágil (flash ou acoplamento por caminho).

## Recomendação

**Opção A.** Mata o `store.rs` do mesmo jeito, sem flash, e vira infra que
qualquer app glacier aproveita. As primitivas `fetch("file://")` + `write_file`
(Parte 1) entram assim mesmo, como o recurso geral que foram pedidas — só não são
elas que carregam a geometria.

## Passos de execução (quando aprovado)

Seguindo a regra do CLAUDE.md (nunca `path`/`[patch]`, sempre publicar e subir):

1. **glacier-ui** (`~/Development/rust/glacier-ui`):
   - `net::send`: desvio `file://` (leitura).
   - `install_write_file` + entrada no prelude (`write_file`).
   - Persistência de geometria via `storage` no builder `GlacierDaemon`
     (Opção A) + `.remember_window_geometry(...)`.
   - Testes (`cargo test`) e, se houver superfície visível, rodar um exemplo.
2. Commit completo na `main` do glacier (antes de publicar).
3. Bump de versão em `glacier-ui/Cargo.toml` (`0.48.0` → `0.49.0`), `cargo
   publish --dry-run` e `cargo publish`.
4. **rustploy** (`crates/rustploy-gui`):
   - Subir `glacier-ui` pra a versão publicada no `Cargo.toml`.
   - Remover `src/app/store.rs`, `save_geometry` e a restauração em
     `main_window_settings`; ligar `.remember_window_geometry(...)`.
   - `luau-lsp analyze` nos scripts tocados (se algum) + `cargo test -p
     rustploy-gui` + rodar o app pra confirmar que a geometria persiste.
