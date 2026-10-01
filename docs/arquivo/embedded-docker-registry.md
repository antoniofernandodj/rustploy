# Rustploy — Registry Docker embutido, Fase 3 (integração com o deploy executor)

Documento auto-contido para retomar esta tarefa numa sessão nova, com
contexto limpo. Tudo que é preciso saber pra implementar está aqui — não
depende do histórico da conversa anterior.

---

## 0. O que é o projeto (contexto mínimo)

Rustploy é um PaaS single-node em Rust (`crates/daemon`, binário `rustployd`),
alternativa ao Dokploy/Coolify. O único cliente é `rustploy-gui` (glacier-ui:
templates XML + lógica em Luau em `views/scripts/`), falando com o daemon via
HTTP/JSON + SSE. Não existe mais TUI (removido em 2026-07-13). `shared` é a
crate com os tipos de protocolo (`Command`/`Response`/`Event`) e modelos.

`Command`/`Response`/`Event` (em `crates/shared/src/protocol.rs`) trafegam
como **JSON** (serde externally-tagged), que é auto-descritivo: variantes e
campos novos podem entrar em qualquer posição, e `skip_serializing_if`/
`serde(default)` são permitidos. A regra antiga de "só no fim, nunca com
skip/default" valia para o wire postcard do listener UDS, removido junto com o
TUI que o usava.

## 1. Onde estamos: registry Docker embutido

O rustployd tem um registry Docker OCI (Distribution API v2) implementado
**dentro do próprio processo** (não é um container `registry:2` orquestrado
— é código Rust em `crates/daemon/src/registry/`). Documentação de
referência completa: `docs/plano-registry-embutido.md` (plano original,
com notas datadas de cada fase concluída) e
`docs/2026-07-13-remocao-tui-e-registry-fase2.md` (resumo legível da Fase 2 +
da remoção do TUI, que aconteceu na mesma sessão).

Fases já implementadas e commitadas no `main`:

- **Fase 1 (núcleo push/pull)**: `docker push`/`docker pull` funcionam contra
  `127.0.0.1:5100`. Storage content-addressable em disco
  (`<db_path>/registry/blobs/sha256/...`), metadados em SQLite
  (`crates/daemon/src/db/registry.rs`, tabelas `registry_repos`,
  `registry_blobs`, `registry_manifests`, `registry_tags`,
  `registry_manifest_refs`).
- **GC**: `crates/daemon/src/registry/gc.rs`, roda via botão na GUI ou job
  diário, remove blobs/manifests órfãos.
- **Fase 2 (Basic auth + exposição pública)**: **toda rota do registry exige
  autenticação Basic** (usuário/senha do jeito que `docker login` já
  entende), **inclusive em loopback** (`127.0.0.1:5100`) — decisão
  deliberada, porque qualquer processo do host consegue falar com loopback,
  então "é local" não é motivo pra confiar. Tokens ficam em
  `registry_tokens` (só o hash SHA-256 do segredo é persistido; o texto
  plano só existe uma vez, na resposta de criação). Escopos `pull`/`push`
  (`push` satisfaz `pull`). Exposição pública opcional via
  `daemon_settings.registry_domain` (precedência banco > config, mesmo
  padrão do e-mail ACME) + `ingress.upsert_route`/`tls.ensure_cert`.

Arquivos-chave já existentes (não mexer neles além do que este plano pede):

- `crates/daemon/src/registry/auth.rs` — verifica `Authorization: Basic` nas
  requisições recebidas pelo registry (server-side). Tem `enum Scope { Pull, Push }`
  e `pub async fn check(req, db, required) -> Result<(), RegistryError>`.
- `crates/daemon/src/db/registry_tokens.rs` — CRUD de tokens
  (`create`, `list`, `revoke`, `verify_scope`, `touch_last_used`). Schema:
  `registry_tokens(id TEXT PK, name TEXT UNIQUE NOT NULL, token_sha256 TEXT,
  scope TEXT, created_at TEXT, last_used_at TEXT)` — **confirmado que `name`
  tem `UNIQUE`** (`crates/daemon/src/db/mod.rs:219`), então dá pra usar
  `ON CONFLICT(name)`.
- `crates/daemon/src/api/handlers/registry.rs` — handlers dos comandos
  `Registry*`, incluindo `token_create`/`token_list`/`token_revoke`. Tem uma
  função privada `generate_secret()` (32 bytes de `/dev/urandom`, hex) — usar
  o mesmo padrão na peça nova.
- `crates/daemon/src/main.rs` (por volta da linha 151-190) — cria o
  `registry_storage` (`Option<Arc<RegistryStorage>>`) se `config.registry.enabled`,
  e registra rota de ingress + cert ACME se houver domínio configurado
  (banco > config, igual ao padrão do e-mail ACME).

## 2. O problema que esta fase (Fase 3) resolve

A Fase 2 introduziu auth obrigatória em toda rota do registry, **inclusive
loopback**. Isso quebrou (silenciosamente, sem ninguém ter percebido até
agora) o único consumidor interno que ainda não mandava credencial nenhuma: o
**deploy executor** do próprio rustploy.

Hoje, `crates/daemon/src/docker/images.rs::pull()` sempre chama
`docker.create_image(options, None, None)` — sem credenciais. Se você criar
um serviço `ServiceSource::Registry { image }` apontando para uma imagem do
**próprio** registry embutido (ex.: `127.0.0.1:5100/hello-world:latest`) e
clicar em Deploy, o pull falha com **401**, porque o Docker Engine do host,
ao puxar a imagem, não manda usuário/senha nenhum.

**Fase 3 = ensinar o executor a se autenticar sozinho quando a imagem é do
próprio registry embutido.** Sem nenhuma ação manual do usuário.

### Esclarecimento importante (perguntado e respondido antes deste documento)

Existem **duas formas válidas** de referenciar uma imagem do registry
embutido, cada uma pra um cenário diferente — e a distinção importa pro
design abaixo:

- **`127.0.0.1:5100/repo:tag`** (ou `localhost:5100/repo:tag`) — usada
  quando quem faz o pull está no **mesmo host** que o daemon (é exatamente o
  caso do deploy executor: o Docker Engine do host chama o registry embutido
  do mesmo host, em loopback). **Com porta**, porque o listener do registry
  só existe em `127.0.0.1:5100` — não é exposto direto na internet.
- **`<domínio-público>/repo:tag`** (ex.: `rustploy.chiquitos.tech/repo:tag`)
  — usada quando quem faz o pull está em **outra máquina** (CI externo, por
  exemplo). **Sem porta** (443/HTTPS implícito), porque o acesso público
  passa pelo **ingress** (o mesmo proxy reverso que roteia os domínios dos
  serviços), que escuta em `:443`/`:80` e encaminha por `Host` header pra
  `127.0.0.1:5100` internamente. `<domínio>:5100/repo:tag` **não funciona** —
  a porta 5100 nunca é exposta publicamente, só em loopback.

O executor de deploy roda no mesmo host que o registry, então o caso comum é
o primeiro formato — mas o design abaixo reconhece os dois (mais o domínio),
pra cobrir também o caso de alguém ter digitado a forma de domínio na spec do
serviço.

## 3. Escopo desta fase

**Dentro do escopo:**
1. Token interno `rp-internal`, gerado automaticamente pelo daemon (nunca
   visto por humano).
2. `docker/images.rs::pull()` aceita credenciais opcionais.
3. Deploy executor detecta se a imagem de um serviço aponta para o registry
   embutido (por prefixo) e, se sim, injeta as credenciais do token interno.
4. Fiação do token pelo processo inteiro (boot → `AppState` → todo lugar que
   cria um `DeployExecutor`).

**Fora de escopo (deliberado, não foi pedido):**
- Seletor de repo/tag no wizard de criação de serviço — o campo de imagem em
  `crates/rustploy-gui/views/service.xml` (`form_control="f_repo_url"`) já é
  texto livre; o usuário já pode digitar `127.0.0.1:5100/repo:tag` nele hoje.
  Esta fase só faz esse deploy **funcionar**, não muda a tela.
- Push-to-deploy automático (`Event::RegistryPush` +
  `ServiceSpec.registry_auto_deploy`) — isso é o resto da "Fase 4" do plano
  original, não pedido agora.

**Critério de aceite**: criar um serviço pela GUI apontando para uma imagem
do registry embutido e conseguir deployar com sucesso (hoje falha com 401).

## 4. Design detalhado

### 4.1 Token interno `rp-internal` (regenerado a cada boot do daemon)

**Por que regenerar a cada boot, em vez de manter fixo**: o rustploy nunca
persiste segredo de token em texto legível — só o hash SHA-256. Manter uma
senha interna fixa exigiria guardar o texto plano em algum lugar recuperável
(uma exceção à regra). Em vez disso, a cada boot o daemon gera uma senha
nova, salva só o hash (substituindo o antigo via upsert) e mantém o texto
plano **só na memória do processo em execução**. Como só o próprio processo
usa essa senha (com ele mesmo), ela mudar a cada restart não afeta nada nem
ninguém — e evita acumular hashes órfãos de boots antigos.

Mudanças:

- **`crates/daemon/src/db/registry_tokens.rs`**:
  - Nova constante `pub const RP_INTERNAL: &str = "rp-internal";`
  - Nova função:
    ```rust
    pub async fn upsert_internal(db: &Db, token_sha256: &str) -> Result<()> {
        let id = format!("rtok_{}", Ulid::new());
        sqlx::query(
            "INSERT INTO registry_tokens (id, name, token_sha256, scope, created_at)
             VALUES (?, ?, ?, 'pull', ?)
             ON CONFLICT(name) DO UPDATE SET
                token_sha256 = excluded.token_sha256,
                created_at   = excluded.created_at",
        )
        .bind(id)
        .bind(RP_INTERNAL)
        .bind(token_sha256)
        .bind(Utc::now())
        .execute(db)
        .await?;
        Ok(())
    }
    ```
  - `list()` ganha `WHERE name != 'rp-internal'` na query (esse token é uma
    peça interna de funcionamento, não algo que o usuário gerencia pela
    tela — não deve aparecer na lista de tokens da sub-aba Registry).
  - Testes unitários novos (seguir o padrão dos testes já existentes no
    mesmo arquivo, que usam um `mem_db()` helper):
    - `upsert_internal` chamado duas vezes com hashes diferentes → uma linha
      só, hash mais recente vence (idempotente).
    - `list()` não retorna a linha `rp-internal` mesmo depois de
      `upsert_internal`, mas continua retornando tokens normais criados via
      `create()`.

- **`crates/daemon/src/api/handlers/registry.rs`** (`token_create`): logo no
  início da função, antes de gerar o segredo, rejeitar nome reservado:
  ```rust
  if name == crate::db::registry_tokens::RP_INTERNAL {
      return RpResponse::err("ReservedName", "\"rp-internal\" é um nome reservado do sistema");
  }
  ```
  (usar o padrão de erro já usado ali perto, `RpResponse::err(code, msg)`).

- **Novo arquivo `crates/daemon/src/registry/internal_token.rs`**:
  ```rust
  //! Token interno usado pelo próprio deploy executor pra puxar imagens do
  //! registry embutido, sem ação manual do usuário. Regenerado a cada boot
  //! (ver comentário em db/registry_tokens.rs sobre por que não é fixo).

  use crate::db::{registry_tokens, Db};
  use anyhow::Result;
  use sha2::{Digest, Sha256};
  use std::io::Read;
  use std::sync::Arc;

  pub async fn ensure(db: &Db) -> Result<Arc<str>> {
      let mut bytes = [0u8; 32];
      std::fs::File::open("/dev/urandom")
          .and_then(|mut f| f.read_exact(&mut bytes))
          .unwrap_or_default();
      let secret = hex::encode(bytes);
      let hash = hex::encode(Sha256::digest(secret.as_bytes()));
      registry_tokens::upsert_internal(db, &hash).await?;
      Ok(Arc::from(secret.as_str()))
  }
  ```
  (mesmo padrão de geração de segredo de `generate_secret()` em
  `api/handlers/registry.rs` — se preferir, extrair isso pra uma função
  compartilhada, mas não é obrigatório; duplicar 4 linhas é aceitável aqui).

- **`crates/daemon/src/registry/mod.rs`**: adicionar `pub mod internal_token;`
  na lista de módulos (`auth`, `error`, `gc`, `http`, `name`, `storage`).

### 4.2 Credenciais no pull

- **`crates/daemon/src/docker/images.rs`**: assinatura de `pull()` ganha um
  parâmetro novo **no fim** (reduz o diff nos call sites por posição):
  ```rust
  pub async fn pull(
      docker: &Docker,
      image: &str,
      service_id: &str,
      deployment_id: &str,
      bus: &EventBus,
      db: &Arc<Db>,
      credentials: Option<bollard::auth::DockerCredentials>,
  ) -> Result<()> {
      ...
      let mut stream = docker.create_image(options, None, credentials);
      ...
  }
  ```
  Único call site hoje: `crates/daemon/src/deploy/executor.rs:176`
  (`images::pull(&self.docker.inner, &image, &svc.id, &dep.id, &self.bus, &self.db).await?;`)
  — vai virar `images::pull(..., creds).await?;` (ver 4.3).

### 4.3 Detecção de prefixo + injeção de credenciais no executor

- **`crates/daemon/src/deploy/executor.rs`**:
  - Novo campo público na struct `DeployExecutor` (a struct já tem `db`,
    `docker`, `ingress`, `bus`, `secrets`, `tls`, `db_path`, `drain_secs`):
    ```rust
    pub registry_internal_token: Option<Arc<str>>,
    ```
    (`Arc` já deve estar importado; `use std::sync::Arc;` se não estiver).

  - Função livre e pura (fácil de testar sem Docker/DB), pode ficar no
    mesmo arquivo, fora do `impl`:
    ```rust
    fn is_embedded_registry_image(image: &str, port: u16, domain: Option<&str>) -> bool {
        if image.starts_with(&format!("127.0.0.1:{port}/")) {
            return true;
        }
        if image.starts_with(&format!("localhost:{port}/")) {
            return true;
        }
        if let Some(d) = domain {
            if image.starts_with(&format!("{d}/")) {
                return true;
            }
        }
        false
    }
    ```
    Testes unitários (`#[cfg(test)] mod tests` no fim do arquivo, ou reaproveitar
    um módulo de testes já existente ali):
    ```rust
    #[test]
    fn reconhece_loopback_com_porta_certa() {
        assert!(is_embedded_registry_image("127.0.0.1:5100/app:v1", 5100, None));
    }
    #[test]
    fn nao_reconhece_porta_errada() {
        assert!(!is_embedded_registry_image("127.0.0.1:9999/app:v1", 5100, None));
    }
    #[test]
    fn reconhece_localhost() {
        assert!(is_embedded_registry_image("localhost:5100/app:v1", 5100, None));
    }
    #[test]
    fn reconhece_dominio_configurado_sem_porta() {
        assert!(is_embedded_registry_image(
            "registry.exemplo.com/app:v1", 5100, Some("registry.exemplo.com")
        ));
    }
    #[test]
    fn nao_reconhece_dominio_com_porta_5100_anexada() {
        // domínio:porta NUNCA é a forma certa (porta só existe em loopback) —
        // garantir que esse caso não bate por acidente com o prefixo do domínio.
        assert!(!is_embedded_registry_image(
            "registry.exemplo.com:5100/app:v1", 5100, Some("registry.exemplo.com")
        ));
    }
    #[test]
    fn imagem_externa_nao_bate() {
        assert!(!is_embedded_registry_image("nginx:latest", 5100, None));
        assert!(!is_embedded_registry_image("ghcr.io/user/app:v1", 5100, Some("registry.exemplo.com")));
    }
    ```

  - Método privado no `impl DeployExecutor`:
    ```rust
    async fn registry_credentials_for(&self, image: &str) -> Option<bollard::auth::DockerCredentials> {
        let token = self.registry_internal_token.as_ref()?;
        let port = shared::RustployConfig::global().registry.port;

        let mut domain = shared::RustployConfig::global().registry.domain.clone();
        if let Ok(Some(d)) = crate::db::daemon_settings::get(
            &self.db,
            crate::db::daemon_settings::KEY_REGISTRY_DOMAIN,
        ).await {
            if !d.trim().is_empty() {
                domain = Some(d);
            }
        }

        if is_embedded_registry_image(image, port, domain.as_deref()) {
            Some(bollard::auth::DockerCredentials {
                username: Some("rp-internal".to_string()),
                password: Some(token.to_string()),
                ..Default::default()
            })
        } else {
            None
        }
    }
    ```
    (repete inline a mesma lógica de precedência banco>config que já existe
    em `main.rs` por volta da linha 172-181 e em
    `api/handlers/get_daemon_settings.rs`/`set_daemon_settings.rs` — **não**
    extrair um helper compartilhado agora, pra não arriscar mexer nesses 2
    call sites existentes sem necessidade. Confirmar o nome exato do import
    do config global — pode ser `shared::RustployConfig` ou só
    `RustployConfig` dependendo do que já está importado no topo do arquivo;
    checar os `use` existentes em `executor.rs` antes de escrever.)

  - No handler do estado `DeployState::PullingImage` (por volta da linha
    169-184 hoje):
    ```rust
    DeployState::PullingImage => {
        let image = self.image_for(dep, svc);
        info!(...);
        let creds = self.registry_credentials_for(&image).await;
        images::pull(&self.docker.inner, &image, &svc.id, &dep.id, &self.bus, &self.db, creds).await?;
        ...
    }
    ```

### 4.4 Fiação do token pelo processo inteiro

O token precisa "nascer" no boot (`main.rs`) e chegar em **todo lugar que
cria um `DeployExecutor { ... }`** — hoje são 6 lugares:

1. `crates/daemon/src/deploy/recovery.rs` — 3 ocorrências de
   `DeployExecutor { ... }` (branches `SwappingIn|Draining`, `Promoting`,
   `RollingBack`), dentro da função `pub async fn recover(db, docker, ingress,
   bus, secrets, tls, db_path, drain_secs)` (parâmetros soltos, não recebe
   `AppState`).
2. `crates/daemon/src/api/deployments.rs` — 1 ocorrência, já dentro de uma
   função que recebe `state: &AppState` (ou similar) e monta
   `campo: state.campo.clone()` pra cada campo.
3. `crates/daemon/src/api/handlers/deploy_start.rs` — idem.
4. `crates/daemon/src/watchdog.rs` — idem.

Mudanças:

- **`crates/daemon/src/deploy/recovery.rs`**: função `recover()` ganha um
  parâmetro novo no fim da lista:
  ```rust
  pub async fn recover(
      db: Arc<Db>,
      docker: Arc<DockerClient>,
      ingress: Arc<IngressController>,
      bus: Arc<EventBus>,
      secrets: Arc<SecretsManager>,
      tls: Arc<TlsManager>,
      db_path: PathBuf,
      drain_secs: u64,
      registry_internal_token: Option<Arc<str>>,
  ) {
  ```
  E os 3 literais `DeployExecutor { ... }` dentro da função ganham
  `registry_internal_token: registry_internal_token.clone(),` (o `.clone()`
  é barato, é um `Arc`).

- **`crates/daemon/src/api/mod.rs`** (struct `AppState` e `AppState::new`):
  novo campo público `pub registry_internal_token: Option<Arc<str>>,` — olhar
  onde `registry_storage` foi adicionado (é o campo mais recente, adicionado
  na Fase 2) e seguir exatamente o mesmo padrão: adicionar o campo na struct,
  adicionar o parâmetro em `new(...)` (no fim da lista de parâmetros, mesma
  posição relativa de `registry_storage` hoje), e setar no construtor.

- **`crates/daemon/src/api/deployments.rs`**,
  **`crates/daemon/src/api/handlers/deploy_start.rs`**,
  **`crates/daemon/src/watchdog.rs`**: nos 3 literais `DeployExecutor { ... }`
  existentes, adicionar a linha
  `registry_internal_token: state.registry_internal_token.clone(),`
  (mesmo padrão dos outros campos ali, tipo `db: state.db.clone()`).

- **`crates/daemon/src/main.rs`**: o bootstrap do token precisa acontecer
  **antes** da chamada a `deploy::recovery::recover(...)` (que hoje é
  chamada por volta da linha 101, **antes** até do bloco que cria
  `registry_storage` na linha ~151 — ou seja, este bootstrap do token
  precisa ser **movido para mais cedo** que o bloco do `registry_storage`,
  logo depois que `db` é conectado). Não depende de `registry_storage` (só
  precisa do `db` e do `config`):
  ```rust
  let registry_internal_token = if config.registry.enabled {
      match registry::internal_token::ensure(&db).await {
          Ok(t) => Some(t),
          Err(e) => {
              tracing::warn!(error = %e, "registry: falha ao gerar token interno, pull de imagens do registry embutido vai falhar até o próximo restart");
              None
          }
      }
  } else {
      None
  };
  ```
  Não deve ser fatal (não usar `?`/`.expect()`) — um erro aqui não deveria
  derrubar o boot do daemon inteiro.

  Depois, passar `registry_internal_token.clone()` para
  `deploy::recovery::recover(...)` (novo argumento no fim) e para
  `AppState::new(...)` (novo argumento, mesma posição relativa de
  `registry_storage`).

## 5. Arquivos tocados (resumo)

| Arquivo | Mudança |
|---|---|
| `crates/daemon/src/registry/internal_token.rs` (novo) | `ensure()` |
| `crates/daemon/src/registry/mod.rs` | `pub mod internal_token;` |
| `crates/daemon/src/db/registry_tokens.rs` | `RP_INTERNAL`, `upsert_internal`, `list()` filtra, testes |
| `crates/daemon/src/api/handlers/registry.rs` | `token_create` rejeita nome reservado |
| `crates/daemon/src/docker/images.rs` | `pull()` ganha parâmetro de credenciais |
| `crates/daemon/src/deploy/executor.rs` | campo novo + `is_embedded_registry_image` (+testes) + `registry_credentials_for` + uso em `PullingImage` |
| `crates/daemon/src/deploy/recovery.rs` | novo parâmetro, thread nos 3 `DeployExecutor{}` |
| `crates/daemon/src/api/mod.rs` | `AppState.registry_internal_token` + parâmetro em `new()` |
| `crates/daemon/src/api/deployments.rs` | novo campo no literal |
| `crates/daemon/src/api/handlers/deploy_start.rs` | novo campo no literal |
| `crates/daemon/src/watchdog.rs` | novo campo no literal |
| `crates/daemon/src/main.rs` | bootstrap do token ANTES de `recover(...)`, thread em `AppState::new(...)` |
| `docs/plano-registry-embutido.md` | nota datada marcando Fase 3 concluída (mesmo padrão das notas de Fase 1/2 já no topo do arquivo) |

## 6. Ordem de implementação sugerida

1. `db/registry_tokens.rs` (const + upsert_internal + list filtrado + testes) → `cargo test -p rustploy --bins registry_tokens`.
2. `registry/internal_token.rs` + `registry/mod.rs` → `cargo check -p rustploy`.
3. `api/handlers/registry.rs` (rejeitar nome reservado) → `cargo check -p rustploy`.
4. `docker/images.rs` (assinatura de `pull`) — vai quebrar compilação no único
   call site até o passo 5 ser feito; ok, é esperado.
5. `deploy/executor.rs` (campo, função pura + testes, helper async, uso em
   `PullingImage`) → `cargo test -p rustploy --bins executor` (ou o nome do módulo de
   teste que acabou usando).
6. `deploy/recovery.rs`, `api/mod.rs`, os 3 call sites de `DeployExecutor{}`,
   `main.rs` → `cargo check --workspace` (deve compilar limpo depois deste
   passo — é aqui que qualquer campo faltando em algum literal vai aparecer
   como erro do compilador, apontando exatamente onde).
7. Nota datada em `docs/plano-registry-embutido.md` (mesmo estilo das notas
   já existentes no topo do arquivo, começando com `> **Fase 3 (...) implementada em <data>**`).

## 7. Verificação (não considerar pronto sem isso)

1. `cargo test -p rustploy --bins` (cobre os testes novos de `upsert_internal`/`list`
   e de `is_embedded_registry_image`) + `cargo check --workspace`.
2. **Smoke manual, obrigatório** (regra do projeto: teste automatizado verde
   não é suficiente pra considerar uma feature pronta — sempre rodar de
   verdade antes):
   - Com o registry habilitado (`[registry] enabled = true` na config, ou já
     habilitado no ambiente de teste), fazer `docker push` de uma imagem de
     teste (ex.: `hello-world`) pro registry embutido, autenticado com um
     token `push` (criar um pela GUI se não tiver um à mão).
   - Criar um serviço no `rustploy-gui` com `ServiceSource::Registry`
     apontando pra essa imagem via `127.0.0.1:5100/<repo>:<tag>` (campo de
     imagem em `service.xml`/`new_service.xml`, é texto livre).
   - Clicar em Deploy. **Antes desta mudança, isso falha com 401** no passo
     de pull. Confirmar que agora o pull funciona e o container sobe.
   - Se houver domínio público configurado no ambiente de teste, repetir
     apontando para `<domínio>/<repo>:<tag>` (sem porta) e confirmar que
     também funciona.
3. Confirmar que uma tentativa de `RegistryTokenCreate` com `name =
   "rp-internal"` pela GUI retorna um erro amigável (não crasha, não cria o
   token), e que a lista de tokens da sub-aba Registry não mostra a entrada
   `rp-internal` mesmo depois do daemon ter subido com o registry habilitado.

## 8. Cuidado com ambiente de teste (aprendido nesta sessão, não repetir o erro)

Ao tentar validar mudanças de config num daemon de teste isolado nesta
sessão, um `RUSTPLOY_CONFIG` apontando pra um TOML customizado **falhou
silenciosamente** (erro de parse do TOML) e o `RustployConfig::load()` caiu
no fallback de tentar `/etc/rustploy/config.toml` (o config do sistema real)
— isso quase causou um daemon de teste tentando bindar nas mesmas portas do
daemon de produção rodando no mesmo host (systemd, `rustployd` ativo,
escutando `127.0.0.1:5100` e `127.0.0.1:9797`). Não houve dano (o bind da API
falhou por porta já em uso, nada foi sobrescrito), mas o cuidado pra próxima
vez que for preciso um daemon de teste local:
- Confirmar que o TOML de teste parseia (`toml::from_str` não falha) antes
  de assumir que `RUSTPLOY_CONFIG` foi respeitado — checar o log de boot
  (`primary=... fallback=...`) pra ver se ele bateu no arquivo certo.
- Usar portas bem distantes das do sistema real (`ss -ltnp` pra conferir o
  que já está escutando antes de escolher).
- Ou simplesmente confiar nos testes unitários (`cargo test -p rustploy --bins`) +
  smoke test contra o daemon real já rodando (com cuidado, só operações
  read-only ou explicitamente aprovadas), em vez de tentar subir uma segunda
  instância isolada — não valeu o risco/esforço para uma mudança de CSS, por
  exemplo (isso foi tentado numa tarefa anterior nesta mesma sessão, para
  verificar visualmente uma correção de layout; foi abortado por esse motivo
  e a verificação feita só por leitura cuidadosa do código + teste de
  template).

## 9. Estado do plano no momento em que este documento foi escrito

Este plano foi discutido e **aprovado pelo usuário** na sessão anterior
(depois de uma primeira versão condensada ter sido rejeitada por "superficial
demais" — a explicação pedagógica completa está preservada acima, nas seções
2-4). A implementação ainda **não começou** — nenhum arquivo dos listados na
seção 5 foi tocado ainda para esta fase. Comece pela seção 6 (ordem de
implementação).

## 10. Outras mudanças de UI feitas na mesma sessão (já commitadas/já corrigidas, contexto)

- Corrigido nesta sessão (ainda **não commitado** até onde se sabe — conferir
  `git status`): botão "Remover" da linha de repositório na sub-aba Registry
  (`crates/rustploy-gui/views/home.xml`) estava sobrepondo o botão "Ver tags"
  porque os dois dividiam uma coluna `col_act` de largura fixa (120px)
  pensada para um único botão (usada em Containers/Images/Volumes/Networks,
  que só têm um botão "Remover" por linha). Corrigido criando uma classe
  nova `.col_act_2` (largura 190px + `spacing: 8`) usada só nessa linha (e no
  cabeçalho `AÇÃO` correspondente), sem mexer na `.col_act` original (ainda
  usada pelas outras sub-abas). Validado com `cargo test -p rustploy-gui
  --test templates_render` (passou); **não foi validado visualmente** por
  Xvfb nesta sessão (motivo: ver seção 8 acima) — vale conferir visualmente
  na próxima vez que abrir a GUI antes de dar por definitivamente resolvido.
