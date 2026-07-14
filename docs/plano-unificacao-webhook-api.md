# Plano: unificar o webhook na porta da API

## Por que mexer nisso

Hoje o daemon sobe **dois servidores HTTP** que fazem essencialmente a mesma
coisa:

| Servidor | Onde escuta | Autenticação | Arquivo |
|----------|-------------|--------------|---------|
| API HTTP/JSON + SSE | `127.0.0.1:9797` (config `[api]`) | Bearer token | `api/http_api.rs` |
| Webhook + callback OAuth | `0.0.0.0:8788` (config `daemon.webhook_port`) | token na URL / state CSRF | `api/webhook_server.rs` |

São dois listeners hyper quase idênticos, duas portas para liberar no firewall,
duas noções de "qual é a URL pública do rustploy". Nada disso é necessário: os
caminhos já são distintos (`/api/...` vs `/webhook/...`), então cabem no mesmo
listener sem colidir.

O sintoma que puxou o fio: não existe lugar nenhum na GUI para ver a URL de
webhook de um serviço — essa tela era do TUI, que foi removido. Os comandos
`GetWebhookUrl` e `RegenerateWebhookToken` continuam vivos no daemon, mas sem
ninguém chamando.

## Uma peça que não é óbvia

O servidor de webhook **não serve só webhooks**. Ele também atende o callback
do OAuth do Gitea (`GET /oauth/gitea/callback`, `webhook_server.rs:50`), e a
setting `webhook_base_url` é usada para montar o `redirect_uri` registrado no
app OAuth do Gitea (`callback_redirect_uri`, linha 236) — não só a URL do
webhook. Por isso a unificação arrasta o OAuth junto; não dá para tratar só o
webhook e deixar o resto quieto.

## Como fica depois

**Um único listener**, o da API. Os dois caminhos públicos passam a ser servidos
por ele:

```
POST {base}/webhook/{service_id}/{token}   → dispara deploy
GET  {base}/oauth/gitea/callback           → conclui o OAuth do Gitea
```

Ambos entram no roteador do `http_api` **antes** da checagem de Bearer token,
porque cada um tem a sua própria autenticação (o token de 192 bits na URL, no
caso do webhook; o `state` CSRF, no caso do OAuth). O resto da API continua
exigindo o Bearer normalmente.

A porta 8788 deixa de existir. O `webhook_server.rs` perde o `run()` (o loop de
listener), mas os *handlers* dele são reaproveitados inteiros pelo `http_api` —
nenhuma lógica de webhook ou de OAuth é reescrita.

### A base URL passa a ser derivada, não configurada

Hoje você digita a URL base à mão em **Settings › Web Server**. Isso sai. O
daemon passa a derivar a base a partir da config que ele já tem:

- Se `api.domain` está definido (a API já termina TLS na própria porta):
  `https://{domain}` — mais `:{porta}` quando a porta não é a 443.
- Caso contrário: `http://{ip_de_saída}:{api.port}`, que é exatamente o fallback
  que o `get_webhook_url.rs` já faz hoje.

Consequência: o campo "Domínio / URL base" some do formulário de Settings, e no
lugar dele a GUI passa a **exibir** (só leitura) a base derivada e a redirect URI
do Gitea — que é o valor que você precisa copiar ao criar o app OAuth lá.

> **Trade-off assumido:** quem rodar o daemon atrás de um proxy reverso externo
> com um domínio que o daemon desconhece perde a capacidade de corrigir a base à
> mão. O caminho suportado passa a ser setar `api.domain` na config. Foi uma
> escolha consciente por simplicidade.

### O que muda de comportamento

O webhook hoje faz bind em `0.0.0.0` (alcançável de fora por padrão) enquanto a
API faz bind em `127.0.0.1`. Unificados, **o webhook fica tão alcançável quanto a
API estiver** — quem quiser receber webhook do GitHub precisa da API exposta
(`api.bind_address` público + `api.token`, ou a API publicada via ingress).

Webhooks já cadastrados em providers externos apontando para `:8788` param de
funcionar e precisam ser reapontados para a nova URL.

## Mudanças, arquivo por arquivo

### Daemon

- **`api/webhook_server.rs`** — remove `run()` (o listener). Os handlers de
  webhook e do callback OAuth viram funções públicas chamadas pelo `http_api`.
- **`api/http_api.rs`** — roteia `POST /webhook/{id}/{token}` e
  `GET /oauth/gitea/callback` antes do gate de Bearer.
- **`main.rs:325-331`** — some o `tokio::spawn` do webhook server.
- **`api/mod.rs`** — `AppState.webhook_port` deixa de fazer sentido; vira a porta
  da API (ou passa a guardar a base derivada, decidido na implementação).
- **`api/handlers/get_webhook_url.rs`** — `build_url` deixa de ler
  `daemon_settings` e passa a usar a base derivada de `[api]`.
- **`db/daemon_settings.rs`** — `KEY_WEBHOOK_BASE_URL` fica sem uso. A constante
  é mantida marcada como deprecada (a linha no banco de instalações existentes
  simplesmente deixa de ser lida).

### Protocolo (`shared/src/protocol.rs`)

- `Command::SetDaemonSettings` perde o campo `webhook_base_url` (sobram
  `acme_email` e `registry_domain`).
- `Response::DaemonSettings` troca `webhook_base_url: Option<String>` por
  `public_base_url: String` — **derivado e só-leitura**, para a GUI exibir a base
  e a redirect URI do OAuth.

### Config

- `daemon.webhook_port` sai de `shared/src/config.rs` (campo, default e o
  override `RUSTPLOY_WEBHOOK_PORT`), de `daemon/src/ports.rs:62` (lista de portas
  reservadas) e de `packaging/config.toml`. Como a config não usa
  `deny_unknown_fields`, um `config.toml` antigo que ainda tenha
  `webhook_port = 8788` continua carregando — a chave é apenas ignorada.

### GUI

- **`views/home.xml`** (Settings › Web Server) — o `TextInput` do domínio vira
  texto só-leitura com a base derivada + a redirect URI do Gitea.
- **`views/scripts/handlers/settings.luau`** — `settings_save` para de mandar
  `webhook_base_url`; `ss_domain` sai.
- **`views/scripts/handlers/connection.luau`** e **`helpers.luau`** —
  `oauth_redirect_uri` passa a usar o `public_base_url` que vem do daemon em vez
  de montar a partir do campo digitado.
- **`views/service.xml`** (aba Deployments) — **volta a UI de webhook**: a URL do
  serviço, um botão de copiar e um de regenerar token, ligando os comandos
  `GetWebhookUrl` / `RegenerateWebhookToken` que já existem e estão órfãos.

### Docs

- `docs/webhooks.md` — a seção "Encontrando a URL no TUI" (linhas 37-48) e a de
  clipboard descrevem um cliente que não existe mais; reescrever para a GUI, a
  porta única e a base derivada.

## Verificação

- `cargo check --workspace` + `cargo test -p daemon` + `cargo test -p shared`.
- `luau-lsp analyze` nos scripts alterados e
  `cargo test -p rustploy-gui --test templates_render`.
- Ponta a ponta: subir o daemon, pegar a URL do webhook pela GUI, disparar
  `curl -X POST <url>` e confirmar que o deploy inicia — e que
  `GET /api/health` **sem** Bearer continua 401 (ou seja, abrir o webhook não
  abriu a API).
