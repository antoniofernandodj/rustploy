# TLS automático via ACME (Let's Encrypt)

Este documento descreve a implementação de HTTPS automático no Rustploy.
O painel em si é acessado via SSH/UDS — o que se resolve aqui são os certificados
das **aplicações hospedadas** pelo daemon.

---

## Como funciona

Quando um serviço com `domain` configurado termina o deploy (`Promoting → Live`),
o daemon dispara em background o provisionamento do certificado via
[ACME HTTP-01](https://letsencrypt.org/docs/challenge-types/#http-01-challenge).

Fluxo completo:

```
Deploy completa (Live)
       │
       ▼
TlsManager::ensure_cert(domain)          ← background task
       │
       ├─ cert já válido? → sai
       │
       ├─ cria/carrega conta ACME
       │       (arquivo: <db_path>/certs/acme-account.json)
       │
       ├─ faz NewOrder para o domínio
       │
       ├─ recebe token HTTP-01 do Let's Encrypt
       │       token → key_auth ficam em ChallengeStore (memória)
       │
       ├─ Let's Encrypt bate em:
       │       http://<domínio>/.well-known/acme-challenge/<token>
       │       → proxy HTTP responde com key_auth
       │
       ├─ LE valida → order fica Ready
       │
       ├─ gera chave privada (rcgen) + CSR
       │
       ├─ finaliza order (envia CSR)
       │
       ├─ baixa chain PEM assinada
       │
       ├─ salva em disco:
       │       <db_path>/certs/<domínio>/cert.pem
       │       <db_path>/certs/<domínio>/key.pem
       │
       └─ carrega no SniResolver (hot-reload sem reiniciar o daemon)
```

---

## Arquitetura interna

### `TlsManager` (`ingress/tls.rs`)

Struct central, criada uma vez no boot e compartilhada via `Arc<TlsManager>`.

| Campo | Tipo | Papel |
|---|---|---|
| `challenges` | `Arc<Mutex<HashMap<token, key_auth>>>` | Compartilhado com o handler HTTP para servir challenges ACME |
| `resolver` | `Arc<SniResolver>` | Seleciona o cert certo por domínio na negociação TLS |
| `server_config` | `Arc<rustls::ServerConfig>` | Criado uma vez; SniResolver é atualizado internamente |
| `cert_dir` | `PathBuf` | `<db_path>/certs/` |
| `acme_config` | `AcmeConfig` | `enabled`, `email`, `directory` (URL do ACME provider) |

### `SniResolver`

Implementa `rustls::server::ResolvesServerCert`. No handshake TLS, recebe o
`server_name` (SNI) do cliente e retorna o `CertifiedKey` correspondente.
Novos certs são injetados via `RwLock` sem recriar o `TlsAcceptor`.

### Proxy HTTP/HTTPS (`ingress/proxy.rs`)

Dois listeners sobem em paralelo quando `acme.enabled = true`:

```
porta 80  (HTTP)
  ├─ /.well-known/acme-challenge/* → serve key_auth do ChallengeStore
  └─ qualquer outro path           → 301 → https://<host><path>

porta 443 (HTTPS / TLS)
  └─ proxy reverso normal (mesmo comportamento do HTTP sem TLS)
```

O listener HTTPS usa `tokio-rustls::TlsAcceptor` sobre TCP.
Se não houver cert para um domínio no momento do handshake, a conexão TLS falha —
comportamento correto enquanto o cert ainda está sendo provisionado.

### Loop de renovação (`main.rs`)

```
a cada 12 horas:
  TlsManager::renew_expiring()
    → percorre certs carregados em memória
    → renova os com arquivo com mais de 60 dias de idade
      (Let's Encrypt emite por 90 dias)
```

---

## Configuração

Em `/etc/rustploy/config.toml` (ou `~/.config/rustploy/config.toml`):

```toml
[ingress]
http_port  = 80
https_port = 443

[ingress.acme]
enabled   = true
email     = "seu@email.com"
directory = "https://acme-v02.api.letsencrypt.org/directory"
```

Para testes (sem emitir certs reais), use o staging do Let's Encrypt:

```toml
[ingress.acme]
enabled   = true
email     = "seu@email.com"
directory = "https://acme-staging-v02.api.letsencrypt.org/directory"
```

---

## Pré-requisitos no servidor

| Requisito | Por quê |
|---|---|
| Porta 80 acessível publicamente | Let's Encrypt bate no HTTP-01 challenge |
| Porta 443 acessível publicamente | HTTPS funcionar para os usuários |
| DNS do domínio apontando para o servidor | LE resolve o domínio antes de validar |
| `acme.email` configurado | Notificações de expiração do LE |

O Rustploy **não gerencia DNS** — o registro A/AAAA precisa estar configurado
antes do primeiro deploy do serviço.

---

## Armazenamento em disco

```
<db_path>/certs/
├── acme-account.json        ← credenciais da conta ACME (não apagar)
├── app.meudominio.com/
│   ├── cert.pem             ← chain completa (cert + intermediários)
│   └── key.pem              ← chave privada RSA/ECDSA
└── outro.dominio.com/
    ├── cert.pem
    └── key.pem
```

O daemon carrega todos os certs presentes no boot. Se um arquivo for corrompido,
o domínio simplesmente não terá cert carregado (warning no log) e o próximo deploy
tentará reprovisionar.

---

## Arquivos modificados na implementação

| Arquivo | O que mudou |
|---|---|
| `crates/daemon/Cargo.toml` | Adicionado `rustls`, `rustls-pemfile`, `tokio-rustls`, `instant-acme`, `rcgen` |
| `crates/daemon/src/ingress/tls.rs` | Reescrito: `TlsManager` + `SniResolver` completos |
| `crates/daemon/src/ingress/proxy.rs` | Listener HTTPS, redirect HTTP→HTTPS, handler de challenge ACME |
| `crates/daemon/src/ingress/mod.rs` | Exporta `TlsManager` |
| `crates/daemon/src/api/mod.rs` | Campo `tls: Arc<TlsManager>` em `AppState` |
| `crates/daemon/src/deploy/executor.rs` | Campo `tls` em `DeployExecutor`; spawn de `ensure_cert` em `Promoting` e `ComposingUp` |
| `crates/daemon/src/deploy/recovery.rs` | Passa `tls` ao reconstruir executores no boot |
| `crates/daemon/src/api/handlers/deploy_start.rs` | Passa `state.tls` ao `DeployExecutor` |
| `crates/daemon/src/api/deployments.rs` | Idem |
| `crates/daemon/src/watchdog.rs` | Idem |
| `crates/daemon/src/main.rs` | Cria `TlsManager`, loop de renovação, passa `tls` ao proxy e ao `AppState` |
