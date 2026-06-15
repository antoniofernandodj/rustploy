# Plano: Rustploy Wire Protocol (RWP)

Este documento registra a decisão de expor uma interface remota administrativa
para o `rustployd` via **Rustploy Wire Protocol (RWP)**: TCP direto, framing
binário e payload em `postcard`, sem HTTP e sem JSON.

O objetivo é permitir um client remoto com baixo consumo de memória, mantendo o
alvo do projeto: daemon abaixo de 50 MB de RAM em idle.

---

## 1. Nome e escopo

O protocolo se chama **Rustploy Wire Protocol**, abreviado como **RWP**.

O nome descreve a camada de transporte própria do Rustploy: um protocolo de fio
binário, mínimo e específico para comunicação entre clients Rustploy e o daemon.
Ele cobre:

- comandos administrativos;
- respostas estruturadas;
- assinatura de eventos;
- stream de eventos do daemon.

O RWP não é uma API pública genérica nem um substituto HTTP. Ele é uma interface
administrativa própria para clients confiáveis do Rustploy.

## 2. Decisão proposta

Adicionar ao daemon um listener TCP dedicado para RPC remoto:

```text
client remoto
    │
    │ RWP sobre TCP + TLS opcional/obrigatório em produção
    ▼
rustployd rwp
    │
    │ Command / Response internos
    ▼
core do daemon
```

O transporte usa mensagens length-prefixed:

```text
request:
  [4 bytes: tamanho u32 little-endian]
  [payload: postcard<Command>]

response:
  [4 bytes: tamanho u32 little-endian]
  [payload: postcard<Response>]
```

Para eventos em stream, o mesmo framing pode carregar `Event`:

```text
event:
  [4 bytes: tamanho u32 little-endian]
  [payload: postcard<Event>]
```

Isso mantém o RWP simples, auditável e barato em RAM. `postcard` não
adiciona custo relevante em idle; o custo real vem de conexões, buffers, TLS e
threads.

## 3. Por que não HTTP para este canal

HTTP com `hyper` é aceitável, mas traz uma base maior: parser HTTP, headers,
estado de conexão, keep-alive, camadas de roteamento e integração TLS via stack
HTTP. Para um canal administrativo controlado, essas capacidades não são
necessárias.

O protocolo binário direto tem vantagens concretas:

- Menos dependências no hot path administrativo.
- Menos alocações por request.
- Framing explícito e barato.
- `Command`, `Response` e `Event` trafegam quase diretamente como tipos Rust.
- Sem ambiguidade de headers, rotas, content-type e status HTTP.

O custo é perder compatibilidade com ferramentas HTTP comuns, como `curl`,
proxies e gateways. Para o Rustploy, esse trade-off é aceitável porque o client
principal é próprio.

## 4. Modelo de execução

Tokio não é necessário para este canal se o objetivo for baixa concorrência
administrativa. O desenho inicial deve ser síncrono:

```text
1 thread accept
N threads de conexão, limitadas por configuração
```

Cada conexão roda um loop bloqueante:

```text
read frame
decode postcard<Command>
executar handler
encode postcard<Response>
write frame
```

Configuração inicial recomendada:

```text
max_connections = 8
thread_stack_size = 256 KiB
max_frame_size = 1 MiB
read_timeout = 15s
write_timeout = 15s
idle_timeout = 120s
```

Threads por conexão são aceitáveis aqui porque a escala esperada é pequena. A
stack virtual default do Linux pode parecer grande, mas o RSS real cresce só nas
páginas tocadas. Ainda assim, usar `std::thread::Builder::stack_size` deixa o
teto explícito e reduz o risco operacional.

## 5. Segurança

Para acesso remoto real, TCP puro não deve ser exposto na internet.

O modo de produção deve usar TLS:

- `rustls` no servidor.
- Certificado configurado pelo usuário ou emitido pelo fluxo ACME do daemon.
- Autenticação no protocolo, antes de aceitar comandos sensíveis.
- Preferência por token estático forte ou mTLS na primeira versão.

Fluxo mínimo de autenticação:

```text
client conecta
TLS handshake
client envia Command::Authenticate { token }
daemon responde Response::Ok ou Response::Err
conexão passa a aceitar comandos administrativos
```

Com mTLS, o token pode ser opcional, mas ainda é útil como segunda camada e para
revogação simples.

Regras obrigatórias:

- Rejeitar frames acima de `max_frame_size`.
- Aplicar timeout de leitura, escrita e idle.
- Limitar conexões simultâneas.
- Não logar secrets, tokens ou payloads completos.
- Fechar a conexão após erro de autenticação.
- Ter opção de bind em `127.0.0.1` por padrão e bind remoto explícito.

## 6. Estimativa de RAM

Base atual informada:

```text
backend + frontend: 38 MB RSS
```

Estimativa de aumento em idle:

| Modelo | Aumento provável | Total provável |
|--------|------------------|----------------|
| TCP síncrono sem TLS | +0,2 a +1 MB | 38,2 a 39 MB |
| TCP síncrono + limite de threads | +0,5 a +2 MB | 38,5 a 40 MB |
| TCP síncrono + rustls | +2 a +5 MB | 40 a 43 MB |
| TCP síncrono + rustls + auth + buffers | +3 a +6 MB | 41 a 44 MB |

Orçamento conservador para a v1 remota:

```text
38 MB atuais
+ 6 MB protocolo remoto com TLS
= 44 MB alvo operacional
```

Isso preserva margem razoável abaixo de 50 MB, desde que os limites de conexão e
buffer sejam aplicados.

## 7. Tipos de protocolo

O protocolo deve reaproveitar os tipos existentes sempre que possível:

```rust
enum RwpFrame {
    Command(Command),
    EventSubscribe(EventFilter),
    Ping,
}

enum RwpReply {
    Response(Response),
    Event(Event),
    Pong { uptime_secs: u64 },
    Error(RwpError),
}
```

Se `Command`, `Response` e `Event` já estiverem estáveis no crate `shared`, o
ideal é serializar esses tipos diretamente em `postcard`. Um envelope só deve
ser adicionado se houver necessidade real de versão, autenticação por sessão ou
mensagens especiais.

## 8. Versionamento

O handshake deve negociar uma versão de protocolo antes dos comandos normais:

```text
client -> Hello { protocol_version, client_version }
daemon -> HelloAck { protocol_version, daemon_version }
```

Na v1:

- Se a versão major não bater, fechar com erro estruturado.
- Se a versão minor do client for menor, o daemon pode aceitar.
- Evitar campos obrigatórios novos sem bump de major.

## 9. Plano de implementação

1. Criar módulo `rwp` no daemon.
2. Definir configuração: bind address, porta, TLS, max frame, max connections.
3. Implementar framing `[u32 LE][payload]` com leitura exata e limite de tamanho.
4. Implementar encoding/decoding com `postcard`.
5. Implementar listener TCP síncrono com thread de accept.
6. Implementar pool simples ou contador de conexões com rejeição quando cheio.
7. Adicionar handshake `Hello` e autenticação.
8. Encaminhar comandos para os handlers já existentes do daemon.
9. Implementar subscribe de eventos usando o `EventBus` existente.
10. Adicionar métricas internas: conexões ativas, bytes recebidos/enviados,
    erros de autenticação e frames rejeitados.
11. Medir RSS antes/depois com `/proc/<pid>/smaps_rollup` e `measure_ram.sh`.

## 10. Critérios de aceite

- Daemon inicia com RWP desabilitado por padrão.
- Ao habilitar sem TLS, bind default é `127.0.0.1`.
- Bind em `0.0.0.0` exige TLS configurado.
- Frames acima do limite são rejeitados sem alocar o payload inteiro.
- Conexões ociosas são encerradas por timeout.
- Com `max_connections = 8`, a nona conexão recebe erro ou é fechada.
- Com TLS habilitado, o aumento de RSS em idle fica abaixo de 6 MB.
- O total esperado permanece abaixo de 44 MB partindo da base atual de 38 MB.

## 11. Medição recomendada

Antes de aceitar a feature, medir em três cenários:

```bash
./measure_ram.sh
```

Cenários:

1. Daemon sem RWP.
2. Daemon com RWP habilitado, sem client conectado.
3. Daemon com RWP habilitado e 8 clients ociosos conectados.

Registrar:

- `VmRSS`
- `VmHWM`
- `PSS` em `/proc/<pid>/smaps_rollup`
- número de threads
- número de conexões ativas

O número que importa para o alvo do projeto é RSS/PSS em idle com a feature
habilitada.

---

## 12. Implementação v1 (entregue)

Esta seção documenta o que foi efetivamente construído e onde a implementação
diverge — conscientemente — do desenho inicial acima.

### 12.1 Envelope do protocolo

Definido em `crates/shared/src/protocol.rs`, reaproveitando `Command`,
`Response` e `Event` diretamente:

```rust
pub const RWP_PROTOCOL_VERSION: u16 = 1;

pub enum RwpFrame {
    Hello { protocol_version: u16, client_version: String },
    Authenticate { token: String },
    Rpc(Command),
    Subscribe { service_id: Option<String> },
    Ping,
}

pub enum RwpReply {
    HelloAck { protocol_version: u16, daemon_version: String, auth_required: bool },
    AuthOk,
    Response(Response),
    Event(Event),
    Pong { uptime_secs: u64 },
    Error(RwpError),
}
```

Framing idêntico ao canal UDS: `[u32 LE length][payload postcard]`.

### 12.2 Ciclo de vida da conexão

```text
client -> Hello { protocol_version, client_version }
daemon -> HelloAck { protocol_version, daemon_version, auth_required }
[ se auth_required ]
client -> Authenticate { token }
daemon -> AuthOk | Error   (Error fecha a conexão)
loop:
    client -> Rpc(Command)        daemon -> Response(...)
    client -> Ping                daemon -> Pong { uptime_secs }
    client -> Subscribe { .. }    daemon -> stream contínuo de Event(...)
```

Diferente do UDS (que fecha após um RPC), o RWP mantém a conexão de comando
viva para múltiplos RPCs sequenciais — o client remoto reusa uma conexão de
comando e abre uma segunda conexão dedicada para o stream de eventos.

### 12.3 Cobertura de comandos

O listener encaminha `RwpFrame::Rpc(cmd)` para o **mesmo** `dispatch()`
(`crates/daemon/src/api/routes.rs`) usado pelo socket local. Logo, **todos** os
comandos da TUI ficam disponíveis remotamente sem código adicional por comando:
projetos (create/list/update/delete/env), serviços (create/list/get/update/
delete/stop/reload), deploys (start/abort/rollback/history), logs/build logs,
métricas (via stream), secrets, webhooks, settings, `DaemonStatus`,
`DeployEngineStatus`, `Ping`. `Subscribe` reusa o `EventBus`.

### 12.4 Desvios conscientes do desenho inicial

- **Tokio em vez de threads síncronas (seção 4).** O daemon já é
  `#[tokio::main]` e todo handler (sqlx, EventBus, bollard) é assíncrono.
  Implementar o listener em Tokio reusa `dispatch()` e `EventBus::subscribe()`
  sem ponte sync↔async (que exigiria um `block_on` por chamada) e sem custo de
  RAM relevante — o runtime já existe. Limites de conexão (`Semaphore`) e de
  frame mantêm o footprint contido.
- **Token estático em vez de TLS na v1 (seção 5).** A v1 entrega TCP + token
  estático opcional (comparação em tempo ~constante). O guard de segurança é:
  bind **não-loopback exige token** — caso contrário o listener não inicia.
  TLS/mTLS via `rustls` permanece como o próximo passo documentado nas seções
  5 e 6; o framing e o envelope já estão prontos para rodar sobre um
  `tokio_rustls::TlsStream` sem mudanças de protocolo.

### 12.5 Configuração

Nova seção `[rwp]` em `RustployConfig` (`crates/shared/src/config.rs`):

```toml
[rwp]
enabled = false          # desabilitado por padrão
bind_address = "127.0.0.1"
port = 8787
token = ""               # vazio = sem auth (apenas loopback)
max_connections = 8
max_frame_size = 1048576 # 1 MiB
read_timeout_secs = 15
idle_timeout_secs = 120
```

Overrides por env (todos centralizados em `apply_env_overrides`):
`RUSTPLOY_RWP_ENABLED`, `RUSTPLOY_RWP_BIND`, `RUSTPLOY_RWP_PORT`,
`RUSTPLOY_RWP_TOKEN`.

### 12.6 Config singleton

Todas as leituras de variáveis de ambiente passam por um singleton
`RustployConfig::global()` (`OnceLock`), carregado uma única vez. Helpers
centralizam `$HOME` (`shared::user_home`, `shared::fallback_data_dir`) e a
resolução de socket do client (`client_socket_candidates`). Daemon e TUI agora
consomem o singleton em vez de ler `std::env::var` ad hoc.

---

## 13. Client gráfico remoto (iced)

Crate `crates/remote-client` (binário `rustploy-remote`), uma GUI
[iced](https://iced.rs) que fala RWP:

- **Tela de conexão:** endereço `host:porta` (default vindo de `rwp_address()`)
  + token opcional.
- **Worker assíncrono** (`src/worker.rs`) exposto como `Subscription` iced,
  mantendo duas conexões RWP: uma de comando (request/response) e uma de eventos
  (`Subscribe`). Comandos da UI viajam por um canal `mpsc`; respostas e eventos
  voltam como mensagens iced.
- **UI** (`src/view.rs`): barra de status (versão/uptime do daemon), painel de
  projetos, painel de serviços com ações **Deploy / Stop / Reload / Rollback**,
  detalhe do serviço e um painel de **eventos ao vivo** (deploy progress, build
  logs, log lines, métricas, mudanças de status).
- **Transporte** (`src/rwp.rs`): handshake, auth e framing.

Execução: `cargo run -p remote-client` (ou binário `rustploy-remote`).
