# Índice: daemon: registry embutido e provedores git

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-daemon/crates/daemon/src/git_providers/

### gitea.rs — Minimal Gitea API client: OAuth2 token exchange/refresh plus the few REST endpoints the UI needs (cu…
struct OAuthTokens { access_token, refresh_token } — Tokens returned by the OAuth token endpoint.
struct TokenResponse { access_token, refresh_token }
struct UserResponse { login, avatar_url }
struct RepoResponse { full_name, clone_url, default_branch, private }
struct BranchResponse { name }
fn client() -> Result<reqwest::Client>
fn base(base_url) -> &str
fn exchange_code(base_url, client_id, client_secret, code, redirect_uri) -> Result<OAuthTokens> — Exchanges an authorization `code` for access/refresh tokens.
fn refresh(base_url, client_id, client_secret, refresh_token) -> Result<OAuthTokens> — Refreshes an expired access token.
fn current_user(base_url, token) -> Result<GitAccount> — Returns the authenticated account.
fn list_repos(base_url, token) -> Result<Vec<GitRepo>> — Lists repositories accessible to the token, paginating until exhausted.
fn list_branches(base_url, token, repo_full_name) -> Result<Vec<GitBranch>> — Lists branches of `owner/repo`.
struct OAuthAppItem { id, name, redirect_uris, client_id }
fn ensure_redirect_uri(base_url, token, client_id, redirect_uri) -> Result<()> — Garante que `redirect_uri` está cadastrada no app OAuth2 identificado por `client_id`.
fn error_for_status(resp) -> Result<reqwest::Response> — Turns a non-2xx response into an error carrying the body for diagnostics.

### github.rs — Minimal GitHub API client: OAuth2 token exchange/refresh plus the few REST endpoints the UI needs (c…
struct OAuthTokens { access_token, refresh_token } — Tokens returned by the OAuth token endpoint.
struct TokenResponse { access_token, refresh_token, error, error_description } — Token endpoint reply.
struct UserResponse { login, avatar_url }
struct RepoResponse { full_name, clone_url, default_branch, private }
struct BranchResponse { name }
fn client() -> Result<reqwest::Client>
fn trim(base_url) -> &str
fn is_dotcom(base_url) -> bool — True when `base_url` points at github.com (the public host) rather than a GitHub Enterprise Server i…
fn web_base(base_url) -> String — Web host, where the OAuth authorize/token endpoints live.
fn api_base(base_url) -> String — REST API host.
fn authorize_url(base_url, client_id_enc, redirect_uri_enc, state_enc) -> String — Builds the authorization URL the client opens in a browser.
fn exchange_code(base_url, client_id, client_secret, code, redirect_uri) -> Result<OAuthTokens> — Exchanges an authorization `code` for an access token.
fn refresh(base_url, client_id, client_secret, refresh_token) -> Result<OAuthTokens> — Refreshes an expired access token (only meaningful when expiring OAuth tokens are enabled; standard …
fn tokens_or_error(t) -> Result<OAuthTokens>
fn current_user(base_url, token) -> Result<GitAccount> — Returns the authenticated account.
fn list_repos(base_url, token) -> Result<Vec<GitRepo>> — Lists repositories accessible to the token, paginating until exhausted.
fn list_branches(base_url, token, repo_full_name) -> Result<Vec<GitBranch>> — Lists branches of `owner/repo`.
fn ensure_redirect_uri(_base_url, _token, _client_id, _redirect_uri) -> Result<()> — GitHub OAuth App callback URLs cannot be edited via the API (unlike Gitea), so there is nothing to a…
fn get(url, token) -> Result<reqwest::Response> — GET with the standard GitHub headers.
fn error_for_status(resp) -> Result<reqwest::Response> — Turns a non-2xx response into an error carrying the body for diagnostics.

### mod.rs — Clients for hosted Git providers.
fn kind_of(p) -> GitProviderKind — Reads a stored provider's kind, defaulting to Gitea for forward-compat with rows written before a ki…
struct OAuthTokens { access_token, refresh_token } — OAuth tokens, unified across providers.
fn current_user(kind, base_url, token) -> Result<GitAccount>
fn list_repos(kind, base_url, token) -> Result<Vec<GitRepo>>
fn list_branches(kind, base_url, token, repo_full_name) -> Result<Vec<GitBranch>>
fn exchange_code(kind, base_url, client_id, client_secret, code, redirect_uri) -> Result<OAuthTokens>
fn ensure_redirect_uri(kind, base_url, token, client_id, redirect_uri) -> Result<()> — Ensures `redirect_uri` is registered on the provider's OAuth app when the provider supports it (Gite…
fn authorize_url(kind, base_url, client_id_enc, redirect_uri_enc, state_enc) -> String — Builds the browser authorization URL.
fn callback_path_segment(kind) -> &'static str — Path segment of the OAuth callback for a provider kind — `gitea` or `github`.
fn usable_token(secrets, p) -> Result<String> — Decrypts the access token (OAuth) or PAT a provider authenticates with.
fn refresh_access_token(db, secrets, p) -> Option<String> — Attempts to refresh an expired OAuth access token, persisting the new pair.

## rustploy-daemon/crates/daemon/src/registry/

### auth.rs — Basic auth do registry OCI embutido — checada em TODA rota (inclusive `GET /v2/`), sem bypass mesmo …
enum Scope { Pull, Push } — Nível de acesso exigido por uma rota — `Push` satisfaz também exigência de `Pull` (um token de escri…
fn scope_satisfies(token_scope, required) -> bool
fn check(req, db, required) -> Result<(), RegistryError> — Verifica `Authorization: Basic <base64(user:pass)>` contra os tokens cadastrados.

### error.rs — Envelope de erro da OCI Distribution Spec: `{"errors":[{"code","message","detail"}]}`, sempre com o …
type RegistryBody = BoxBody<Bytes, Infallible> — Body unificado das respostas do registry — mesmo padrão de `ApiBody` em `api/http_api.rs`.
enum RegistryError { NameInvalid, NameUnknown, BlobUnknown, BlobUploadUnknown, BlobUploadInvalid, DigestInvalid, ManifestUnknown, ManifestInvalid, ManifestBlobUnknown, Unauthorized, Internal }
impl RegistryError
  fn code() -> &'static str
  fn status() -> StatusCode
  fn message() -> String
  fn into_response() -> Response<RegistryBody>
impl From<anyhow::Error>, From<std::io::Error>, From<sqlx::Error> for RegistryError

### gc.rs — Garbage collection do registry: libera do disco o que nenhuma tag alcança.
const UPLOAD_TTL — Idade mínima de um arquivo órfão em `uploads/` para o GC apagar — folga generosa para uploads legiti…
struct GcResult { blobs_removed, bytes_freed }
fn run(db, storage) -> anyhow::Result<GcResult>
(3 testes)

### http.rs — Rotas HTTP da OCI Distribution API v2 — dispatch manual (hyper cru não tem router): `match`/`rsplit_…
fn run(db, storage, port) — Bind (loopback only); retorna em caso de falha (log + listener desabilitado, daemon continua) — mesm…
fn serve(listener, db, storage) — Loop de accept, separado de `run` para os testes de integração poderem bindar em `127.0.0.1:0` e des…
fn route(req, db, storage) -> Response<RegistryBody>
fn dispatch(path, query, method, req, db, storage)
fn ping()
fn catalog(db)
fn tags_list(db, repo_name)
fn blob(method, storage, repo_name, digest)
fn blob_body(file) -> RegistryBody — Stream do arquivo em frames de 64KB — nunca bufferiza um blob inteiro em memória (camadas de imagem …
fn blob_upload(method, req, db, storage, repo_name, uuid, query)
fn start_or_monolithic_upload(req, db, storage, repo_name, query)
fn patch_upload(req, storage, repo_name, uuid)
fn put_upload(req, storage, db, repo_name, uuid, query)
fn delete_upload(storage, uuid)
fn storage_err(e) -> RegistryError
fn upload_accepted_response(repo_name, uuid, written) -> Response<RegistryBody>
fn created_blob_response(repo_name, digest_hex) -> Response<RegistryBody>
fn manifest(method, req, db, storage, repo_name, reference)
fn resolve_manifest_digest(db, repo_id, reference) -> Result<String, RegistryError>
fn get_manifest(db, storage, repo_name, reference, head_only)
fn put_manifest(req, db, storage, repo_name, reference) — `PUT /v2/<repo>/manifests/<ref>`: valida que os blobs referenciados existem e grava o manifest e a t…
fn extract_refs(value) -> Result<Vec<String>, RegistryError> — Extrai os digests referenciados por um manifest: `manifests[].digest` para um index/manifest-list (m…
fn delete_manifest_route(db, repo_name, reference)
fn collect_body(req) -> Result<Bytes, RegistryError>
fn empty_body() -> RegistryBody
fn json_response(status, value)
fn query_param(query, key) -> Option<String> — Extrai `key` da query string, com percent-decoding (mesmo esquema de `api::webhook_server::percent_d…
fn percent_decode(s) -> String
(8 testes)

### internal_token.rs — Token interno usado pelo próprio deploy executor pra puxar imagens do registry embutido, sem ação ma…
fn ensure(db) -> Result<Arc<str>>

### mod.rs — Registry Docker OCI Distribution API v2 embutido — push/pull, GC e Basic auth por token.

### name.rs — Validação de `<name>`, `<reference>` (tag) e `<digest>` da OCI Distribution Spec — implementada como…
fn is_valid_name(name) -> bool — Valida `<name>` contra a gramática OCI distribution-spec.
fn is_alnum(b) -> bool
fn is_valid_component(c) -> bool
fn is_valid_tag(tag) -> bool — Valida `<reference>` quando não é um digest — regex da spec: `[a-zA-Z0-9_][a-zA-Z0-9._-]{0,127}` (má…
fn parse_digest(s) -> Option<&str> — Extrai os 64 hex lowercase de `sha256:<hex>`.
(4 testes)

### storage.rs — CAS (content-addressable store) do registry: blobs em disco, sessões de upload em memória com hash i…
struct BlobInfo { digest, size }
enum StorageError { UnknownUpload, DigestMismatch, Io }
struct UploadSession { file, tmp_path, hasher, written }
struct RegistryStorage { root, uploads, commit_lock } — Blob store content-addressable + registro de sessões de upload em voo.
impl RegistryStorage
  fn new(root) -> std::io::Result<Self>
  fn lock_commit() -> tokio::sync::MutexGuard<'_, ()> — Ver `commit_lock`.
  fn digest_path(digest_hex) -> PathBuf
  fn blob_exists(digest_hex) -> bool
  fn open_blob(digest_hex) -> std::io::Result<tokio::fs::File>
  fn blob_len(digest_hex) -> std::io::Result<u64>
  fn start_upload() -> std::io::Result<String> — Inicia uma sessão de upload; retorna o ID (ULID) usado na URL (`Docker-Upload-UUID` / `/v2/<name>/bl…
  fn write_chunk(id, data) -> Result<u64, StorageError> — Append de um chunk; retorna o total de bytes já escritos (para o header `Range: bytes=0-<written-1>`…
  fn finalize_upload(id, expected_digest_hex) -> Result<BlobInfo, StorageError> — Finaliza: confere digest, fsync, rename atômico para o CAS.
  fn cancel_upload(id) -> Result<(), StorageError>
  fn write_blob_direct(data) -> Result<BlobInfo, StorageError> — Escreve um blob a partir de bytes já em memória, sem sessão de upload — usado pelo `PUT` de manifest…
  fn read_blob(digest_hex) -> std::io::Result<Vec<u8>> — Lê um blob inteiro em memória — usado para servir manifests (pequenos, já têm teto de 4 MiB no PUT) …
  fn sweep_orphan_files(live) -> std::io::Result<(u64, u64)> — Sweep do GC: remove do CAS todo arquivo cujo nome (digest) não está em `live` — cobre blobs órfãos e…
  fn clean_stale_uploads(max_age) -> std::io::Result<(u64, u64)> — Remove de `uploads/` arquivos órfãos: sem sessão ativa no mapa (sobra de restart do daemon — as sess…
impl std::fmt::Display, std::error::Error for StorageError
(5 testes)
