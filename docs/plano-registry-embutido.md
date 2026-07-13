# Registry Docker embutido no rustployd

> Plano de implementação. Investigação feita em 2026-07-12 sobre o estado atual do daemon.
>
> **Fase 1 (núcleo push/pull) implementada em 2026-07-13** — `crates/daemon/src/registry/` (`name.rs`/`error.rs`/`storage.rs`/`http.rs`) + `crates/daemon/src/db/registry.rs` + bloco `[registry]` em `shared/src/config.rs`. Validada com `docker push`/`pull` reais (incluindo multi-arch via `buildx --push`, manifest index). Loopback only (`127.0.0.1:5100`), sem autenticação, sem ingress/TLS, sem integração com o deploy executor — isso continua Fase 2/3, não implementado. Retomar a partir daqui quando for atacar a próxima fase.
>
> **GC implementado em 2026-07-13** — `crates/daemon/src/registry/gc.rs` (2 fases: `db::registry::gc_metadata` apaga manifests pendurados/refs/blobs órfãos numa transação; sweep do CAS + `uploads/` órfãos > 24 h no storage), `Command::RegistryGc` → `Response::RegistryGcResult { blobs_removed, bytes_freed }`, botão "Executar GC" na sub-aba Registry e job diário em `main.rs` (junto do trim do event_log). A trava contra corrida do plano virou a `commit_lock` do `RegistryStorage`, compartilhada via `AppState.registry_storage` (o storage agora é criado no `main.rs`, não dentro de `registry::http::run`), segurada pelos 3 pontos de commit do `http.rs` (upload monolítico, finalize de PUT de blob, PUT de manifest desde a validação das refs) e pelo GC inteiro (snapshot→sweep). Além do plano original: o GC também remove manifests **pendurados** (sem tag e não referenciados por index) — sem isso, republicar uma tag nunca liberaria as camadas antigas. Janela aceita (single-admin): GC no meio de um push com blobs finalizados e manifest ainda não enviado apaga esses blobs e o push falha (BLOB_UNKNOWN) — basta repetir o push.
>
> **Nota de implementação (achado real, não previsto no plano original)**: o header `Range` das respostas de upload de blob (`202 Accepted` do POST/PATCH) usa o formato próprio da OCI Distribution Spec — `Range: <start>-<end>` **sem** o prefixo de unidade `bytes=` do `Range` HTTP genérico (RFC 7233). Usar `bytes=0-N` faz o cliente `docker` CLI falhar com `"expected integer"` ao fazer parse ingênuo dos números — só foi pego pelo smoke test real, não pelos testes automatizados (que reproduziam o valor esperado, não a spec real). Confirmar esse detalhe se algum dia reimplementar do zero.
>
> **Sub-aba Registry na GUI implementada em 2026-07-13** (parte do que a Fase 3 previa) — `crates/rustploy-gui/views/home.xml` (5ª sub-aba de Docker), `views/scripts/{fmt,handlers}/registry.luau`, `Command::RegistryStatus/RegistryRepoList/RegistryTagList/RegistryTagDelete/RegistryRepoDelete` em `protocol.rs` + handler em `api/handlers/registry.rs`. Somente leitura + delete (metadados; sem GC de blob órfão ainda). **Bug real encontrado e corrigido no mesmo dia**: `registry_manifests` tinha `digest` como PK global — quando dois repos recebiam push do MESMO conteúdo (manifest byte-idêntico, digest igual), o segundo `insert_manifest` roubava a posse (`repo_id`) do primeiro via `ON CONFLICT`, quebrando `list_repos` (tamanho zerado), `delete_manifest` (404) e **o `docker pull` real** do repo que perdia a posse (`manifest unknown`). Corrigido trocando a PK pra composta `(repo_id, digest)` — um manifest pode agora pertencer a vários repos de forma independente, sem afetar as rotas OCI em `registry/http.rs` (só `db/registry.rs` + o schema em `db/mod.rs` mudaram). Teste de regressão em `db/registry.rs::manifests_com_mesmo_digest_nao_colidem_entre_repos`.
>
> **Fase 2 (auth + exposição pública) implementada em 2026-07-13** — Basic auth por token, obrigatória em TODA rota do registry (inclusive `GET /v2/`, sem bypass nem em loopback): `crates/daemon/src/registry/auth.rs` (`Scope::Pull`/`Push`, `Push` satisfaz `Pull`), `RegistryError::Unauthorized` com `WWW-Authenticate: Basic realm="rustploy-registry"`, checagem em `registry/http.rs::route()` antes do dispatch. Tokens: tabela `registry_tokens` (nome único, só o SHA-256 do segredo é persistido) + `crates/daemon/src/db/registry_tokens.rs` + `Command::RegistryTokenCreate/List/Revoke` (segredo em texto plano só na criação, uma vez). Exposição pública: `daemon_settings.registry_domain` (precedência sobre `[registry] domain` do config, mesmo padrão do e-mail ACME) + `ingress.upsert_route`/`ensure_cert` tanto no boot (`main.rs`) quanto ao mudar em runtime (`set_daemon_settings.rs`), com `RUSTPLOY_REGISTRY_DOMAIN` como override de env. GUI: campo de domínio em Settings > Web Server, seção "Tokens de acesso" na sub-aba Registry, janela `new_registry_token_window.xml` (segredo exibido uma vez, `docker login` pronto pra copiar). **Sem token interno `rp-internal` ainda** — só faz sentido quando a Fase 3 (deploy executor) o consumir. Validado com smoke test real completo: `docker login`/push/pull autenticados funcionando, 401 sem credencial, escopo `pull` corretamente barrado de fazer push, revogação derrubando acesso na hora, `SetDaemonSettings` com domínio registrando a rota no ingress de verdade (confirmado via `curl` batendo no domínio configurado).

## Contexto e motivação

Hoje o rustploy só consegue obter imagens de duas formas: **build local a partir de Git** (`ServiceSource::Git`) ou **pull de um registry externo** (`ServiceSource::Registry { image }` → Docker Hub, GHCR etc.). Não há como um pipeline de CI externo *entregar uma imagem pronta* diretamente ao servidor sem depender de um registry de terceiros (com limites de rate, credenciais extras e latência) ou de rodar um `registry:2` avulso configurado à mão.

Um registry embutido fecha esse ciclo: `docker push registry.meudominio.com/app:v1` a partir do CI → serviço aponta para essa imagem → deploy. É o mesmo papel que o Dokploy resolve com um registry self-hosted, mas aqui **dentro do próprio `rustployd`**, sem container extra — coerente com a filosofia do projeto (proxy reverso embutido em hyper, ACME embutido, sem nginx/traefik externos).

Investigação confirmou a infraestrutura já disponível:

- **Servidores HTTP hyper "crus" já são o padrão da casa** — `api/http_api.rs` (API JSON+SSE), `api/webhook_server.rs` e `ingress/proxy.rs` são todos `hyper::server::conn::http1` + `service_fn`, sem framework. O registry segue o mesmo molde.
- **Ingress roteia por Host e faz stream do corpo** (`ingress/proxy.rs::forward` repassa `Incoming` sem bufferizar) — blobs de GBs passam pelo proxy sem custo de memória. HTTP→HTTPS redirect e desafio ACME HTTP-01 já tratados no listener HTTP.
- **TLS/ACME por domínio já existe** (`ingress/tls.rs::ensure_cert`) — um domínio dedicado do registry ganha certificado Let's Encrypt automaticamente, então o `docker` CLI de qualquer máquina fala com ele sem configurar `insecure-registries`.
- **Pull no deploy não passa credenciais hoje** — `docker/images.rs::pull` chama `create_image(options, None, None)`; o terceiro parâmetro (`Option<DockerCredentials>`) do bollard existe justamente para isso e será usado para autenticar o pull da imagem do registry embutido.
- **Docker Engine dispensa TLS para loopback**: registries em `127.0.0.0/8` são tratados como inseguros-permitidos por padrão. O pull local (feito pelo Engine do próprio host) pode usar `127.0.0.1:<porta>/repo:tag` via HTTP puro, sem depender de DNS/certificado — funciona inclusive antes de existir domínio configurado.
- **`webhook_server.rs:83` já mostra o gancho de auto-deploy**: chamar `handlers::deploy_start::handle(state, service_id)` — o push-to-deploy reutiliza exatamente isso.
- Deps: `hex`, `ulid`, `hyper`, `sqlx` já presentes; falta apenas **`sha2`** (digest de conteúdo).

## Decisão central

**Implementar a OCI Distribution Spec (registry HTTP API v2) em Rust dentro do daemon**, e não gerenciar um container `registry:2`. Motivos:

- "Embutido" de verdade: zero containers de infraestrutura, um único binário, storage no `db_path` já existente.
- A superfície da spec necessária para `docker push`/`docker pull` é pequena (~10 rotas) e o modelo é simples (conteúdo endereçado por sha256).
- Controle total: auth integrada aos tokens do rustploy, eventos no `event_bus` a cada push (habilita push-to-deploy e UI ao vivo), GC integrado ao SQLite — nada disso é possível de forma limpa orquestrando o `registry:2` (que exigiria htpasswd, webhooks HTTP próprios e GC via CLI dele).

## Visão geral da arquitetura

```
CI externo ── docker push ──> ingress :443 (TLS/ACME, Host: registry.dominio.com)
                                   │ proxy HTTP/1.1 (stream)
                                   ▼
                     registry listener 127.0.0.1:5100  (novo, dentro do rustployd)
                                   │
                    ┌──────────────┼──────────────────┐
                    ▼              ▼                   ▼
              auth (tokens)   storage CAS          SQLite (repos,
              Basic auth      <db_path>/registry/  tags, manifests,
                              blobs/sha256/…       refs, tokens)
                                   ▲
Docker Engine local ── pull 127.0.0.1:5100/repo:tag ──┘   (deploy executor,
                                                           HTTP loopback, com creds)
```

- **Novo módulo `crates/daemon/src/registry/`**: `mod.rs` (estado compartilhado), `http.rs` (rotas da Distribution API), `storage.rs` (blob store + sessões de upload), `auth.rs`, `gc.rs`.
- Listener **sempre em loopback** (`127.0.0.1:<porta>`); a exposição pública é exclusivamente via ingress (rota Host → loopback), nunca bind direto em 0.0.0.0. Isso mantém TLS, e futuramente rate-limit, num lugar só.
- O mesmo repositório é acessível pelos dois nomes — `registry.dominio.com/app:v1` (push externo) e `127.0.0.1:5100/app:v1` (pull local) — porque o host **não** faz parte da identidade do repo dentro do registry; só o path (`app`) identifica.

## Config (`crates/shared/src/config.rs`)

Novo bloco `[registry]` em `RustployConfig` (com `#[serde(default)]` no struct de config — a restrição "sem serde default" vale só para os tipos do protocolo postcard, não para TOML):

```toml
[registry]
enabled = true            # default: false (opt-in)
port = 5100               # bind sempre em 127.0.0.1
domain = "registry.exemplo.com"   # opcional; registra rota no ingress + cert ACME
storage_dir = ""          # default: <db_path>/registry/
```

Overrides de env: `RUSTPLOY_REGISTRY_ENABLED`, `RUSTPLOY_REGISTRY_PORT`, `RUSTPLOY_REGISTRY_DOMAIN`.

No boot (`main.rs`), quando habilitado: sobe o listener, e se `domain` estiver setado faz `ingress.upsert_route(domain, vec!["127.0.0.1:5100"], "rp-registry")` + `tls.ensure_cert(domain)` (mesmo padrão que a API usa). O `domain` também pode vir/ser alterado via banco (`daemon_settings`, como o email ACME) para ser configurável pela GUI sem editar TOML — chave `registry_domain`, precedência sobre o config.

## Superfície da OCI Distribution API a implementar

Tudo sob `/v2/` no listener do registry. `<name>` validado contra a regex da spec (`[a-z0-9]+(?:[._-][a-z0-9]+)*(?:/…)*`, máx. 255 chars) — **obrigatório** para impedir path traversal no storage.

| Rota | Uso |
|---|---|
| `GET /v2/` | ping de versão + desafio de auth (401 → `docker login`) |
| `HEAD/GET /v2/<name>/blobs/<digest>` | existência/download de blob (com `Docker-Content-Digest`) |
| `POST /v2/<name>/blobs/uploads/` | inicia upload → `202` com `Location`/`Docker-Upload-UUID`; suporta `?digest=` (monolítico) e `?mount=<digest>&from=<repo>` (dedupe entre repos) |
| `PATCH /v2/<name>/blobs/uploads/<uuid>` | append de chunk (stream para arquivo em `uploads/<uuid>`, hash sha256 incremental em memória); responde `202` com `Range` |
| `PUT /v2/<name>/blobs/uploads/<uuid>?digest=sha256:…` | finaliza: confere digest, `rename()` atômico para o CAS |
| `DELETE /v2/<name>/blobs/uploads/<uuid>` | cancela upload |
| `HEAD/GET /v2/<name>/manifests/<ref>` | por tag ou digest; devolve o **media type original** e `Docker-Content-Digest` |
| `PUT /v2/<name>/manifests/<ref>` | valida JSON (limite 4 MiB), confere que config/layers (ou manifests filhos, no caso de index/manifest list) existem, grava, atualiza tag |
| `DELETE /v2/<name>/manifests/<digest>` | remove manifest (e tags que apontam para ele) |
| `GET /v2/_catalog` e `GET /v2/<name>/tags/list` | listagem (com paginação `n`/`last` da spec) |

Detalhes de conformidade que o `docker` CLI exige: header `Docker-Distribution-API-Version: registry/2.0` em toda resposta; erros no envelope JSON `{"errors":[{"code","message","detail"}]}` com os códigos da spec (`BLOB_UNKNOWN`, `MANIFEST_UNKNOWN`, `DIGEST_INVALID`, `NAME_INVALID`, `UNAUTHORIZED`, …); media types Docker schema2 **e** OCI (manifest, index/manifest list) aceitos e devolvidos byte-a-byte como recebidos (o digest é do corpo bruto).

Sessões de upload vivem só em memória (`HashMap<uuid, UploadSession>` com o hasher incremental) + arquivo parcial em disco; restart do daemon descarta sessões (o `docker push` simplesmente tenta de novo). GC limpa arquivos órfãos em `uploads/` com mais de 24 h.

## Storage

**Blobs em disco, metadados no SQLite.**

Disco (CAS — content-addressable store):
```
<db_path>/registry/
  blobs/sha256/<2 primeiros hex>/<digest completo>   # blobs E manifests (manifest também é blob)
  uploads/<uuid>                                      # parciais
```

SQLite (migration nova em `db/mod.rs`, wrappers em `db/registry.rs`):
```sql
registry_repos     (id TEXT PK, name TEXT UNIQUE NOT NULL, created_at TEXT)
registry_blobs     (digest TEXT PK, size INTEGER, created_at TEXT)
registry_manifests (digest TEXT PK, repo_id TEXT, media_type TEXT, size INTEGER, created_at TEXT)
registry_tags      (repo_id TEXT, tag TEXT, manifest_digest TEXT, updated_at TEXT,
                    PRIMARY KEY (repo_id, tag))
registry_manifest_refs (manifest_digest TEXT, blob_digest TEXT,
                        PRIMARY KEY (manifest_digest, blob_digest))  -- config+layers+filhos
registry_tokens    (id TEXT PK, name TEXT UNIQUE, token_sha256 TEXT, scope TEXT,
                    created_at TEXT, last_used_at TEXT)
```

`registry_manifest_refs` é a base do GC por contagem de referência: blob sem nenhuma ref e sem tag → removível.

## Autenticação

**Basic auth** (não o token service Bearer da spec — Basic é suportado nativamente pelo `docker login` e é o que o `registry:2` + htpasswd usa; o esquema Bearer só vale a pena com multi-tenancy real):

- `GET /v2/` sem credencial → `401` + `WWW-Authenticate: Basic realm="rustploy-registry"`; o CLI pede login e passa a mandar o header.
- Credencial = **token de registry**: nome (username) + segredo aleatório de 32 bytes (password), gerado pelo daemon, exibido **uma única vez** na criação. Armazenado só o SHA-256 (token de alta entropia dispensa argon2).
- Escopos: `pull` (só leitura) e `push` (leitura+escrita). Um token interno `rp-internal` (escopo pull) é gerado/rotacionado automaticamente pelo daemon para o executor de deploy.
- Auth exigida **sempre**, inclusive em loopback — o listener em 127.0.0.1 é alcançável por qualquer processo/usuário do host, então não há bypass por origem.

## Integração com o deploy

1. **Pull autenticado** — `docker/images.rs::pull` ganha parâmetro `Option<bollard::auth::DockerCredentials>`. O executor (`deploy/executor.rs`), ao ver `ServiceSource::Registry` cuja imagem aponta para o registry embutido (prefixo `127.0.0.1:<porta>/`, `localhost:<porta>/` ou `<domain>/`), injeta as credenciais do token interno. Imagens externas seguem como hoje (sem creds).
2. **Referência recomendada na spec do serviço**: `127.0.0.1:<porta>/repo:tag` — o pull é local (Engine → loopback), não depende de DNS/certificado e funciona mesmo sem domínio configurado. A GUI oferece o seletor (repo/tag vindos do catálogo) e monta essa referência sozinha; quem digita o domínio público também funciona (Engine puxa via ingress/TLS).
3. Nenhuma mudança no state machine do deploy — do ponto de vista do executor é um registry como outro qualquer.

## Push-to-deploy

A cada `PUT /v2/<name>/manifests/<tag>` bem-sucedido, o registry publica `Event::RegistryPush { repo, tag, digest }` no `event_bus` (GUI atualiza ao vivo) e um gancho procura serviços com `ServiceSource::Registry` cuja imagem case com `<host-qualquer-do-registry>/<repo>:<tag>` **e** que tenham opt-in de auto-deploy, disparando `handlers::deploy_start::handle` (mesmo mecanismo do webhook_server).

Opt-in: novo campo `ServiceSpec.registry_auto_deploy: bool`. ⚠️ `ServiceSpec` viaja em postcard → campo **sem** `serde(default)`/`skip`, adicionado **no fim do struct**, com migração de dados no load do banco se a spec for persistida como JSON (conferir na implementação: specs no SQLite são JSON, então default no load do DB é ok; o que não pode é quebrar o wire `Command`/`Response`/`Event`).

Fluxo de CI resultante: `docker login registry.dominio.com` (token push) → `docker build` → `docker push` → rustploy redeploya sozinho. Sem webhook HTTP, sem Git no servidor.

## Protocolo / Commands (UDS postcard + HTTP JSON automático)

Novas variantes **sempre no fim** dos enums (postcard é posicional), sem `skip_serializing_if`/defaults:

- `Command::RegistryStatus` → `Response::RegistryStatus { enabled, port, domain, repo_count, blob_count, storage_bytes }`
- `Command::RegistryRepoList` → `Response::RegistryRepos(Vec<RegistryRepo { name, tag_count, size_bytes }>)`
- `Command::RegistryTagList { repo }` → `Response::RegistryTags(Vec<RegistryTag { tag, digest, size_bytes, pushed_at }>)`
- `Command::RegistryTagDelete { repo, tag }` / `Command::RegistryRepoDelete { repo }` → `Response::Ok`
- `Command::RegistryGc` → `Response::RegistryGcResult { blobs_removed, bytes_freed }`
- `Command::RegistryTokenCreate { name, scope }` → `Response::RegistryTokenCreated { name, secret }` (única vez que o segredo aparece)
- `Command::RegistryTokenList` / `Command::RegistryTokenRevoke { name }`
- `Event::RegistryPush { repo, tag, digest }`

Handlers em `api/handlers/registry_*.rs`, roteados em `api/routes.rs::dispatch()`. Como a API HTTP (`POST /api/rpc`) e o SSE reusam `dispatch()`/`Event`, **a GUI ganha tudo isso de graça**, sem transporte novo.

## GUI (rustploy-gui) e TUI

- **GUI**: nova sub-aba **"Registry"** na view Docker (`home.xml`), ao lado de Containers/Images/Volumes/Networks: lista de repos → tags (digest, tamanho, data), botões de deletar tag/repo, "Executar GC", uso de disco total; seção de **tokens** (criar com escopo, listar, revogar — modal mostra o comando `docker login` pronto para copiar); campo de domínio do registry em Settings (grava em `daemon_settings`). Lógica em `views/scripts/handlers/registry.luau` (validar com `luau-lsp analyze`), refresh ao vivo via `Event::RegistryPush` no consumidor SSE.
- **TUI**: fase posterior, escopo mínimo (listagem de repos/tags em Settings ou aba própria) — não bloqueia o MVP.
- **IaC export/import**: fora de escopo — imagens são dados (não config declarativa) e tokens são segredos de exibição única. Documentar essa decisão no doc de IaC.

## Garbage collection

- `DELETE` de tag/manifest só mexe em metadados (rápido, seguro).
- `RegistryGc` (manual via UI, + job diário junto do trim do event_log): dentro de uma transação, apaga blobs com refcount 0 (sem entrada em `registry_manifest_refs` e sem manifest/tag), remove arquivos do CAS, limpa `uploads/` órfãos > 24 h. **Trava simples contra corrida**: GC adquire um `Mutex` que o finalizador de upload/manifest PUT também segura (operações curtas — só o commit de metadados, não o streaming).

## Fases de implementação

**Fase 1 — núcleo (push/pull manual funcionando)**
`registry/storage.rs` (CAS + sessões de upload + hash incremental), `registry/http.rs` (todas as rotas, erros da spec), migration + `db/registry.rs`, bloco `[registry]` no config, boot no `main.rs` (loopback only, ainda sem auth). Critério: `docker push 127.0.0.1:5100/hello:v1` e `docker pull` de volta funcionam do host; `docker manifest`/imagens multi-arch (index) aceitas.

**Fase 2 — auth + exposição pública**
`registry/auth.rs` + `registry_tokens` + token interno; rota no ingress + `ensure_cert(domain)`; `docker login` de máquina externa via HTTPS. Critério: push externo autenticado com cert válido; 401 correto sem credencial.

**Fase 3 — integração rustploy**
Credenciais no `images::pull` (bollard `DockerCredentials`) + detecção de prefixo no executor; Commands/Responses/handlers; sub-aba Registry + tokens na GUI. Critério: criar serviço apontando para imagem do registry embutido pela GUI e deployar.

**Fase 4 — ciclo completo**
GC (comando + job diário), `Event::RegistryPush` + push-to-deploy com `registry_auto_deploy`, polimento (paginação catalog, `last_used_at` de tokens, uso de disco no status).

## Testes

- **Unit (storage)**: upload em chunks com digest incremental, digest inválido rejeitado, rename atômico, mount entre repos, validação de `<name>` (casos de traversal `../`), GC não apaga blob referenciado.
- **Integração HTTP (sem Docker)**: simular a sequência exata do cliente docker (POST→PATCH→PUT blob, PUT manifest, GET por tag e por digest, HEAD, 401→Basic) com `reqwest` contra o listener em porta efêmera — cobre a conformidade sem depender do Engine.
- **Smoke manual antes de dar por pronto** (regra do projeto): `docker push`/`pull` reais com o CLI, `docker login`, e um deploy fim-a-fim pela GUI. Opcional: rodar a suíte oficial de conformidade da distribution-spec (`github.com/opencontainers/distribution-spec/tree/main/conformance`) apontando para o listener.

## Riscos e limitações conhecidas

- **Disco**: sem quota no MVP — um CI descontrolado enche o disco. Mitigação: uso de disco visível na UI + GC; quota configurável fica como melhoria futura.
- **HTTP/1.1 only no ingress**: ok — os clientes docker/containerd falam 1.1; nada a fazer.
- **Restart durante push**: sessão de upload morre; o docker CLI re-tenta a camada do zero. Aceitável single-node.
- **Uploads concorrentes do mesmo blob**: os dois finalizam, o segundo `rename` sobre o mesmo path do CAS é idempotente (mesmo conteúdo, mesmo digest).
- **Manifest referenciando blob de outro repo sem mount**: exigir refs presentes no CAS global (não por repo) simplifica e é seguro num registry single-admin; anotar como diferença deliberada vs. registries multi-tenant.
- **Postcard**: qualquer descuido com posição/default nas novas variantes quebra o wire TUI↔daemon — revisar contra a regra já registrada antes de mergear.

## Fora de escopo (futuro)

Quota de storage por repo; proxy/pull-through cache do Docker Hub; replicação; UI de vulnerability scan; suporte a `Range` em GET de blob (resumable pull — o docker não exige); TUI completa.
