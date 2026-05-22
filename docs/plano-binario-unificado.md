# Plano: Unificar daemon + client em um binário `rustploy`

## Motivação

Como daemon e client sempre executam na mesma máquina, faz sentido distribuir
um único binário em vez de dois. O IPC via Unix socket é mantido (o daemon
roda em background, o client conecta via socket).

## Interface final

```bash
rustploy              # abre o TUI client (padrão)
rustploy --client     # idem, explícito
rustploy -c           # idem, abreviado
rustploy --daemon     # sobe o daemon (bloqueia até ^C)
rustploy -d           # idem, abreviado
```

---

## Estrutura de crates resultante

```
crates/
  shared/     (sem alteração)
  daemon/     (vira lib + bin; lib expõe pub async fn run())
  client/     (vira lib + bin; lib expõe pub async fn run())
  rustploy/   (novo — thin binary que despacha entre os dois)
```

---

## Mudanças arquivo por arquivo

### 1. `crates/daemon/Cargo.toml`

Adicionar seção `[lib]` e manter `[[bin]]` para `rustployd` (compatibilidade
com systemd):

```toml
[lib]
name = "daemon"
path = "src/lib.rs"

[[bin]]
name = "rustployd"
path = "src/main.rs"
```

### 2. `crates/daemon/src/lib.rs` *(novo)*

Move todo o conteúdo atual de `main.rs` para cá:

- `mod api; mod db; mod deploy; …` → permanecem **privados**
- `async fn main()` → `pub async fn run() -> anyhow::Result<()>`
- Funções auxiliares (`init_logging`, `resolve_socket_path`,
  `resolve_data_path`, `fallback_dir`) → ficam aqui como `fn` privadas

### 3. `crates/daemon/src/main.rs` *(simplificado)*

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    daemon::run().await
}
```

---

### 4. `crates/client/Cargo.toml`

Adicionar seção `[lib]` (o `[[bin]]` pode ser mantido para dev ou removido):

```toml
[lib]
name = "client"
path = "src/lib.rs"

[[bin]]
name = "rustploy-client"
path = "src/main.rs"
```

### 5. `crates/client/src/lib.rs` *(novo)*

Move todo o conteúdo atual de `main.rs` para cá:

- `mod app; mod events; mod transport; mod ui;` → **privados**
- `async fn main()` → `pub async fn run() -> anyhow::Result<()>`
- Constante `TICK_MS` e funções `run()`, `process_pending()`,
  `load_initial_data()`, `resolve_socket()`, `fallback_socket()` → ficam aqui

### 6. `crates/client/src/main.rs` *(simplificado)*

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    client::run().await
}
```

---

### 7. `crates/rustploy/Cargo.toml` *(novo crate)*

```toml
[package]
name = "rustploy"
version = "0.1.0"
edition = "2024"

[[bin]]
name = "rustploy"
path = "src/main.rs"

[dependencies]
daemon = { path = "../daemon" }
client = { path = "../client" }
tokio  = { version = "1", features = ["full"] }
anyhow = "1"
```

### 8. `crates/rustploy/src/main.rs` *(novo)*

```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    match std::env::args().nth(1).as_deref() {
        Some("-d") | Some("--daemon") => daemon::run().await,
        Some("-c") | Some("--client") | None => client::run().await,
        Some(unknown) => {
            eprintln!("Uso: rustploy [-d|--daemon] | [-c|--client]");
            eprintln!("Argumento desconhecido: {unknown}");
            std::process::exit(1);
        }
    }
}
```

---

### 9. `Cargo.toml` *(workspace root)*

```toml
[workspace]
members = [
    "crates/daemon",
    "crates/client",
    "crates/shared",
    "crates/rustploy",   # ← adicionar
]
resolver = "2"
```

---

## Binários resultantes

| Binário          | Como buildar              | Uso                         |
|------------------|---------------------------|-----------------------------|
| `rustployd`      | `cargo build -p daemon`   | systemd / serviço de sistema |
| `rustploy`       | `cargo build -p rustploy` | uso geral (client + daemon)  |

### Build de release

```bash
cargo build --release -p rustploy
# Gera: target/release/rustploy
```

---

## Decisões de design

| Decisão | Motivo |
|---------|--------|
| Módulos internos permanecem privados | Apenas `run()` fica `pub`; encapsulamento preservado |
| `rustployd` sobrevive como binário separado | Compatibilidade com units systemd existentes |
| Novo crate `rustploy` em vez de misturar em `daemon` | `daemon` não carrega deps do client (ratatui/crossterm) desnecessariamente quando usado como lib |
| Sem `clap` para o dispatch | Apenas um flag; parsing manual é suficiente e evita dep extra |
| Zero reescrita de lógica | Só `main()` → `pub async fn run()`; todo o código permanece onde está |
