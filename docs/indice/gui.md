# Índice: rustploy-gui (Rust)

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> `.gv`: `<screen>`/`<component>`, `props`, `imports` (componentes), `script`, `views` (valores de `if="{view}" equals=`) e `handlers` (chamados em `on_*=`).
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-gui/

### build.rs — Build script: 1. Stages os **logos** dos blueprints (só imagens) para `$OUT_DIR` — o `src/embedded.r…
const LOGO_EXTS — Extensões de imagem que os logos de blueprint usam (o resto da pasta — `docker-compose.yml`/`templat…
const LOGO_MAX_DIM — Alvo do redimensionamento dos logos raster: a maior dimensão é reduzida para no máximo isto, preserv…
fn main()
fn stage_blueprint_logos() — Leva os logos de `assets/blueprint-logos/**` para `$OUT_DIR/blueprint_logos/**`, **espelhando a estr…
fn copy_images(src, dst) — Percorre recursivamente `src` e leva os logos para `dst`, preservando o caminho relativo.
fn stage_raster(src, dst) — Lê um logo raster, reduz para no máx [`LOGO_MAX_DIM`] (Lanczos3) re-encodando como PNG, e grava em `…
fn downscale_png(bytes) -> Option<Vec<u8>> — Decodifica `bytes`, e — se a maior dimensão passar de [`LOGO_MAX_DIM`] — reduz preservando a proporç…
fn ext_lower(path) -> Option<String>
fn is_logo(path) -> bool — Qualquer arquivo de logo (raster, vetor ou os formatos raros) — decide o que entra no staging.
fn is_raster(path) -> bool — Um logo raster que o `image` sabe decodificar (as features habilitadas no `Cargo.toml`).

## rustploy-gui/src/agent/

### actions.rs — Índice das ações dispatcháveis da UI.
struct Acao { nome, origem } — Uma ação e onde ela mora.
fn nome_da_acao(linha) -> Option<&str> — Nome da função global declarada nesta linha, se houver.
fn do_arquivo(origem, conteudo) -> Vec<Acao> — Varre um arquivo Luau e devolve suas ações.
fn fontes() -> Vec<(String, String)> — Todos os pares `(caminho, conteúdo)` dos scripts Luau.
fn fontes() -> Vec<(String, String)>
fn index() -> Value — O índice, pronto para sair pela API.
(3 testes)

### catalog.rs — `GET /agent/schema` — o documento de descoberta.
fn schema() -> Value — O documento de descoberta servido em `GET /agent/schema`: o que a API de agente faz, suas rotas e ex…
(2 testes)

### client.rs — Cliente HTTP para o daemon rustploy remoto.
const TIMEOUT — Teto por requisição ao daemon.
type HttpsClient = Client<HttpsConnector<HttpConnector>, Full<Bytes>>
enum RemoteError { Transport, Status, Decode } — Falha ao falar com o daemon remoto, já separada no que o agente precisa distinguir: problema de tran…
struct Remote { http } — Cliente reutilizável (mantém o pool de conexões vivo entre requisições).
impl Remote
  fn new() -> Result<Self, String> — Monta o cliente.
  fn rpc(session, cmd) -> Result<serde_json::Value, RemoteError> — Executa um `Command` no daemon: `POST /api/rpc`.
  fn upload_archive(session, service_id, filename, zip) -> Result<serde_json::Value, RemoteError> — Sobe um zip para `POST /api/services/<id>/archive`.
fn response_kind(v) -> Option<&str> — Nome da variante de uma `Response` externamente tagueada (`{"Projects":[…]}` → `"Projects"`), ou da …
fn response_payload(v) -> Option<&serde_json::Value> — Payload de uma `Response` com campos, ou `None` para variante unitária.
fn response_error(v) -> Option<(String, String)> — `Some((code, message))` quando a resposta é `Response::Err`.
impl std::fmt::Display for RemoteError
(3 testes)

### handoff.rs — Arquivo de handoff: como um agente na mesma máquina descobre esta API.
const FILE — Nome do arquivo dentro do data dir.
fn path() -> PathBuf — Caminho completo do handoff.
fn generate_token() -> String — Token de acesso desta execução: 32 bytes de CSPRNG em hex.
fn write(addr, token, remote) -> std::io::Result<()> — Grava (ou regrava) o handoff.
fn remove() — Remove o handoff.
fn restrict_permissions(f) -> std::io::Result<()>
fn restrict_permissions(_f) -> std::io::Result<()>
(1 testes)

### mod.rs — API de agente — um servidor HTTP local que empresta a sessão desta janela.
const STARTED — Se esta execução chegou a subir o servidor.
const DEFAULT_ADDR — Endereço padrão.
const ENV_VAR — Variável que desliga a API (`RUSTPLOY_AGENT_API=off`) ou troca o endereço (`RUSTPLOY_AGENT_API=127.0…
fn spawn(session, ui) — Sobe o servidor numa thread própria, com um runtime tokio próprio.
fn cleanup() — Apaga o arquivo de handoff.
fn configured_addr() -> Option<SocketAddr> — Endereço a usar, ou `None` quando a API foi desligada por env var.
fn resolve_addr(raw) -> Option<SocketAddr> — Regra de resolução do endereço, separada da leitura da env var para poder ser testada.
(5 testes)

### routes.rs — Servidor hyper da API de agente e os handlers de cada rota.
const MAX_BODY — Teto do corpo de uma requisição.
const POLL_INTERVAL — De quanto em quanto tempo o `wait` de um deploy reconsulta o daemon.
const MAX_WAIT — Teto do `timeout_s` aceito em `POST /agent/deploys`.
struct Ctx { session, remote, ui, token, addr } — Estado compartilhado por todas as conexões.
struct ApiFail { status, code, message } — Erro já no formato em que ele sai pela rede.
impl ApiFail
  fn new(status, code, message) -> Self
  fn desconectado() -> Self — A janela não está conectada a daemon nenhum.
  fn bad_request(message) -> Self
  fn into_response() -> Response<Full<Bytes>>
fn serve(addr, session, ui) -> Result<(), String> — Sobe o listener e serve até o processo morrer.
fn accept_loop(listener, ctx) -> Result<(), String> — O laço de aceitação, separado de [`serve`] para os testes poderem exercitar o roteamento sem gerar t…
fn bind(preferido) — Tenta o endereço pedido; se a porta estiver ocupada (outro app, ou uma instância anterior ainda ence…
fn remote_url(ctx) -> Option<String>
fn watch_session(ctx) — Mantém o campo `remote_url`/`connected` do handoff em dia.
fn handle(req, ctx) — Roteia uma requisição da API de agente: liveness sem token, depois o gate de token, as rotas de dado…
fn not_found(metodo, caminho) -> ApiFail
fn build_log_path(p) -> Option<String> — `/agent/deploys/<id>/logs` → `<id>`.
fn archive_path(p) -> Option<String> — `/agent/services/<id>/archive` → `<id>`.
fn check_token(req, esperado) -> Result<(), ApiFail> — Bearer da API de agente (não o do daemon).
fn ct_eq(a, b) -> bool
fn rpc(ctx, cmd) -> Result<Value, ApiFail> — Executa um `Command` no daemon e já converte `Response::Err` em falha.
fn rpc_expect(ctx, cmd, variante) -> Result<Value, ApiFail> — Como [`rpc`], mas exige que a resposta seja a variante esperada e devolve o payload dela.
fn status(ctx) -> Result<Response<Full<Bytes>>, ApiFail> — `GET /agent/status` — a janela, o daemon e a fila de deploys num lugar só.
fn services(ctx) -> Result<Response<Full<Bytes>>, ApiFail> — `GET /agent/services` — índice achatado projeto→serviço.
fn snapshot(ctx) -> Result<Value, ApiFail> — `Snapshot` devolve `Response::Snapshot(String)` — JSON **dentro** de uma string, não um objeto.
fn deploys(ctx, query) -> Result<Response<Full<Bytes>>, ApiFail> — `GET /agent/deploys` — últimos deploys com o desfecho já resolvido.
fn start_deploy(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/deploys` — dispara um deploy e, com `wait`, só responde quando ele terminou.
fn resolve_service(ctx, corpo) -> Result<String, ApiFail> — Descobre o `service_id` do corpo: aceita o id direto ou o nome do serviço.
fn wait_for_outcome(ctx, service_id, deployment_id, limite) -> Result<(Value, bool), ApiFail> — Poll até o deployment chegar a um estado terminal (ou o prazo acabar).
fn build_logs(ctx, deployment_id, query) -> Result<Response<Full<Bytes>>, ApiFail> — `GET /agent/deploys/<id>/logs` — build log com cursor.
fn build_log_lines(ctx, deployment_id) -> Result<Vec<String>, ApiFail> — Texto de cada linha do build log, na ordem em que foi gravada.
fn ingress_routes(ctx) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/rpc` — qualquer `Command` do protocolo, sem tradução.
fn ingress_reconcile(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/ingress/reconcile` — recalcula as rotas a partir dos containers que existem de fato, se…
fn rpc_passthrough(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail>
fn connect(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/connect` — entra na sessão pela própria tela de login.
fn disconnect(ctx) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/disconnect` — sai da sessão.
fn ui_state(ctx, query) -> Response<Full<Bytes>> — `GET /agent/ui` — o que a janela está mostrando agora.
fn ui_action(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/ui/action` — dispara qualquer ação da UI pelo nome.
fn ui_context(ctx, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/ui/context` — escreve chaves no contexto da janela.
fn upload_archive(ctx, service_id, req) -> Result<Response<Full<Bytes>>, ApiFail> — `POST /agent/services/<id>/archive` — sobe um zip local para o serviço.
fn compact_summary(sum) -> Value — `DeploymentSummary` → linha compacta com o desfecho resolvido.
fn compact_deployment(dep) -> Value — `Deployment` → o que interessa a quem só quer saber como acabou.
fn outcome_ok(estado) -> Value — `true` = no ar, `false` = falhou, `null` = ainda não decidiu.
fn is_terminal(dep) -> bool
fn failure_reason(dep) -> Value — A causa da falha, tirada do `states_log`.
fn service_index(snapshot) -> Vec<Value> — Achata o snapshot no índice de serviços que um agente precisa para agir: id, nome, projeto, status e…
fn status_label(status) -> Value — `ServiceStatus` é externally-tagged e só a variante `Error` tem campo — vira `"Running"` ou `"Error:…
fn source_label(source) -> Value — `ServiceSource` achatado no que identifica a origem, sem arrastar o `compose.content` inteiro (dezen…
fn read_json(req) -> Result<Value, ApiFail>
fn json_response(status, body) -> Response<Full<Bytes>>
fn num_param(query, chave) -> Option<usize> — Parâmetro numérico da query string.
fn str_param(query, chave) -> Option<String> — Parâmetro textual da query string.
impl From<RemoteError> for ApiFail
(22 testes)

### servers.rs — Os servidores que o usuário já usou nesta máquina.
struct Saved { url, token } — Um servidor conhecido.
fn storage_path() -> PathBuf — Arquivo do `storage` do glacier para a janela principal.
fn list() -> Vec<Saved> — Lê a lista salva.
fn token_for(url) -> Option<String> — Token salvo para uma URL, comparando de forma tolerante à barra final — a mesma normalização que a s…
fn as_json() -> Value — A listagem que sai pela API: URL e se há token guardado — nunca o token.

### session.rs — A sessão da GUI (URL + token do daemon remoto), compartilhada com o servidor da API de agente.
struct Session { base_url, token } — Conexão viva com um daemon rustploy, do ponto de vista da API de agente.
struct Espelho { session, context } — O que a thread da API enxerga da janela: a sessão (quando conectada) e um espelho do contexto do mot…
struct SharedSession(Arc<RwLock<Espelho>>) — Handle compartilhado.
impl SharedSession
  fn get() -> Option<Session>
  fn context_key(key) -> Option<String> — Uma chave do contexto da janela, como o motor a tem agora.
  fn context() -> ContextMap — Cópia do contexto inteiro.
  fn sync_from_context(ctx) -> bool — Relê o contexto do motor, espelha-o e atualiza a sessão se algo mudou.
impl Session
  fn from_context(ctx) -> Option<Self> — Extrai a sessão do contexto do motor, ou `None` se não houver conexão.
(6 testes)

### ui.rs — Controle da própria janela — o que antes só um clique alcançava.
const REDIGIDAS — Chaves do contexto que **nunca** saem pela API.
const TIMEOUT_CONNECT — Quanto esperar o `connect()` da camada Luau concluir.
const POLL — De quanto em quanto tempo o `connect` reconsulta o espelho.
enum ConnectOutcome { Conectado, Recusado, Timeout } — Resultado de um `connect` pedido pela API.
fn connect(ui, session, url, token) -> ConnectOutcome — Entra na sessão: preenche o formulário de login e aciona o botão Connect, exatamente como um usuário…
fn disconnect(ui) — Sai da sessão.
fn state(session) -> Value — Estado da janela que interessa a quem a dirige de fora.
fn keys(session, pedidas) -> Value — Chaves avulsas do contexto, para o que o resumo curado não cobre.
fn all_keys(session) -> Value — Todas as chaves do contexto, com os segredos redigidos.
(3 testes)

## rustploy-gui/src/app/

### mod.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates and rendered by the pu…
const FONT_REGULAR — Fontes embutidas (JetBrains Mono): registradas no builder do daemon e usadas como `default_font` de …
fn run() -> iced::Result — Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.
fn platform_specific() -> window::settings::PlatformSpecific — `application_id` only exists on the Linux (X11/Wayland) variant of `PlatformSpecific`; other platfor…
fn platform_specific() -> window::settings::PlatformSpecific
const FONT_BOLD

## rustploy-gui/src/

### assets.rs — Runtime asset location.
const MARKER — A file that must exist under any valid asset base — used as the probe.
const SYSTEM_PREFIX — System-wide install prefix used by the Debian package (see the `deb` metadata in `Cargo.toml`).
fn locate_and_chdir() — Finds the asset base directory and `chdir`s into it so all the CWD-relative asset paths resolve.
fn find_base() -> Option<PathBuf>
fn has_marker(base) -> bool

### embedded.rs — Assets embutidos no binário — modo standalone (só em builds de release).
const VIEWS — `views/`: templates `.gv`, estilos `styles/*.gss`, `styles/theme.json` e os scripts Luau em `scripts…
const ICONS — `assets/icons/`: ícones SVG referenciados por `<svg src="assets/icons/…">`.
const BLUEPRINTS — Logos dos blueprints, referenciados via o `{logo}` data-driven do catálogo do daemon (`assets/bluepr…
fn luau_sources() -> Vec<(String, &'static str)> — Fontes Luau embutidas, como `(caminho relativo, conteúdo)`.
fn route(path) -> Option<&'static File<'static>> — Roteia um caminho lógico para a árvore embutida + o caminho relativo a ela (a chave que `include_dir…
fn not_found(path) -> io::Error
struct EmbeddedAssets — [`AssetSource`] servindo a árvore de assets embutida no binário.
impl AssetSource for EmbeddedAssets: read_bytes, read_to_string, exists, modified, supports_reload
(2 testes)

### main.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates and rendered by the pu…
fn main() -> iced::Result

### manifest_zip.rs — Ponte Lua ↔ Rust para o `.zip` do Infra as Code.
const YML_ENTRY — Os dois nomes de entrada que o export grava e o import espera na raiz do zip.
fn write_manifest_zip(path, yaml, toml) -> std::io::Result<()> — Cria um `.zip` em `path` com exatamente `rustploy.yml` e `rustploy.vars.toml` na raiz — sem diretóri…
fn read_manifest_zip(path) -> Result<(String, String), String> — Valida e lê o `.zip` importado: **exatamente** um `*.yml`/`*.yaml` e um `*.toml`, na raiz, e nada ma…
fn install(lua) -> mlua::Result<()> — Instala os globais `manifest_zip_write` e `manifest_zip_read` na VM Luau.
const TOML_ENTRY
(4 testes)

## rustploy-gui/tests/fixtures/

### compose_host.gv — Fixture do teste fmt_service_detail.rs: tela mínima que roda o fmt/service_detail.luau e exibe um re…
<screen "fixture">
script: compose_host.luau

### compose_host.luau — Fixture do teste `fmt_service_detail.rs`: exercita `compose_host` e `internal_url` de `fmt/service_d…
function init()

### tempo.gv — Fixture do teste fmt_time.rs: tela mínima que roda o fmt/time.luau e exibe o resultado.
<screen "fixture">
script: tempo.luau

### tempo.luau — Fixture do teste `fmt_time.rs`: exercita o `fmt/time.luau` de verdade, através do motor, e deixa cad…
function init()

## rustploy-gui/tests/

### fmt_service_detail.rs — O `fmt/service_detail.luau` (`compose_host` e `internal_url`) rodando no motor de verdade.
fn boot() -> GlacierUI
(2 testes)

### fmt_time.rs — O `fmt/time.luau` rodando no motor de verdade.
fn boot() -> GlacierUI
fn offset_local_segundos() -> i64 — Offset local em segundos, perguntado ao sistema — a mesma fonte que o `localtime` do Luau consulta.
fn hora_deslocada(hms, offset) -> String — `HH:MM:SS` + offset, com a virada de dia descartada (só as horas importam).
(3 testes)

### templates_render.rs — Headless validation: every template parses, every screen/tab evaluates and builds an iced element tr…
fn boot() -> GlacierUI — Boots the engine the way `main.rs` does, but from the workspace root so the workspace-relative templ…
fn cd_ws_root() — Cd's to the workspace root (idempotent — safe alongside `boot`).
fn comentarios_fora(src) -> String — Remove os blocos `<!-- … -->` para que "a primeira tag" seja a primeira tag de verdade: todo templat…
(13 testes)

## rustploy-gui/vendor/iced_tiny_skia/src/

### engine.rs
struct Engine { text_pipeline, raster_pipeline, vector_pipeline }
impl Engine
  fn new() -> Self
  fn draw_quad(quad, background, transformation, pixels, clip_mask, clip_bounds)
  fn draw_text(text, transformation, pixels, clip_mask, clip_bounds)
  fn draw_primitive(primitive, transformation, pixels, clip_mask, clip_bounds)
  fn draw_image(image, _transformation, _pixels, _clip_mask, _clip_bounds)
  fn trim()
fn into_color(color) -> tiny_skia::Color
fn into_transform(transformation) -> tiny_skia::Transform
fn rounded_rectangle(bounds, border_radius) -> tiny_skia::Path
fn maybe_line_to(path, x, y)
fn arc_to(path, x_from, y_from, x_to, y_to, radius)
fn smoothstep(a, b, x) -> f32
fn rounded_box_sdf(to_center, size, radii) -> f32
fn adjust_clip_mask(clip_mask, bounds)

### geometry.rs
enum Geometry { Live, Cache }
struct Cache { text, images, primitives, clip_bounds }
impl Cached for Geometry: load, cache
struct Frame { clip_bounds, transform, stack, primitives, images, text }
impl Frame
  fn new(bounds) -> Self
impl geometry::frame::Backend for Frame: width, height, size, center, fill, fill_rectangle, stroke, stroke_rectangle, fill_text, stroke_text, push_transform, pop_transform, draft, paste, translate, rotate, scale, scale_nonuniform, into_geometry, draw_image, draw_svg
fn transform_rectangle(rectangle, transform) -> (Rectangle, Radians)
fn convert_path(path) -> Option<tiny_skia::Path>
fn into_paint(style) -> tiny_skia::Paint<'static>
fn into_fill_rule(rule) -> tiny_skia::FillRule
fn into_stroke(stroke) -> tiny_skia::Stroke

### layer.rs
type Stack = layer::Stack<Layer>
struct Layer { bounds, quads, primitives, images, text }
impl Layer
  fn draw_quad(quad, background, transformation)
  fn draw_paragraph(paragraph, position, color, clip_bounds, transformation)
  fn draw_editor(editor, position, color, clip_bounds, transformation)
  fn draw_text(text, position, color, clip_bounds, transformation)
  fn draw_text_raw(raw, transformation)
  fn draw_text_group(text, clip_bounds, transformation)
  fn draw_text_cache(text, clip_bounds, transformation)
  fn draw_image(image, transformation)
  fn draw_raster(image, bounds, clip_bounds, transformation)
  fn draw_svg(svg, bounds, clip_bounds, transformation)
  fn draw_primitive_group(primitives, clip_bounds, transformation)
  fn draw_primitive_cache(primitives, clip_bounds, transformation)
  fn damage(previous, current) -> Vec<Rectangle>
impl graphics::Layer for Layer: with_bounds, bounds, flush, resize, reset, start, end, merge
enum Item { Live, Group, Cached }
impl Item<T>
  fn transformation() -> Transformation
  fn clip_bounds() -> Rectangle
  fn as_slice() -> &[T]
impl Default for Layer

### lib.rs
struct Renderer { default_font, default_text_size, layers, engine } — A [`tiny-skia`] graphics renderer for [`iced`].
impl Renderer
  fn new(default_font, default_text_size) -> Self
  fn layers() -> &[Layer]
  fn draw(pixels, clip_mask, viewport, damage, background_color)
impl core::Renderer for Renderer: start_layer, end_layer, start_transformation, end_transformation, fill_quad, reset, allocate_image
impl core::text::Renderer for Renderer: default_font, default_size, fill_paragraph, fill_editor, fill_text
impl graphics::text::Renderer for Renderer: fill_raw
impl graphics::geometry::Renderer for Renderer: new_frame, draw_geometry
impl graphics::mesh::Renderer for Renderer: draw_mesh, draw_mesh_cache
impl core::image::Renderer for Renderer: load_image, measure_image, draw_image
impl core::svg::Renderer for Renderer: measure_svg, draw_svg
impl renderer::Headless for Renderer: new, name, screenshot
impl compositor::Default for Renderer

### primitive.rs
enum Primitive { Fill, Stroke }
impl Primitive
  fn visible_bounds() -> Rectangle — Returns the visible bounds of the [`Primitive`].

### raster.rs
struct Pipeline { cache }
impl Pipeline
  fn new() -> Self
  fn load(handle)
  fn dimensions(handle) -> Option<Size<u32>>
  fn draw(handle, filter_method, bounds, opacity, pixels, transform, clip_mask)
  fn trim_cache()
struct Cache { entries, hits }
impl Cache
  fn allocate(handle)
  fn trim()
struct Entry { width, height, pixels }

### settings.rs
struct Settings { default_font, default_text_size } — The settings of a [`Compositor`].
impl Default, From<graphics::Settings> for Settings

### text.rs
struct Pipeline { glyph_cache, cache }
impl Pipeline
  fn new() -> Self
  fn load_font(bytes)
  fn draw_paragraph(paragraph, position, color, pixels, clip_mask, transformation)
  fn draw_editor(editor, position, color, pixels, clip_mask, transformation)
  fn draw_cached(content, bounds, color, size, line_height, font, align_x, align_y, shaping, pixels, clip_mask, transformation)
  fn draw_raw(buffer, position, color, pixels, clip_mask, transformation)
  fn trim_cache()
fn draw(font_system, glyph_cache, buffer, position, color, pixels, clip_mask, transformation)
fn from_color(color) -> Color
struct GlyphCache { entries, recently_used, trim_count }
impl GlyphCache
  fn new() -> Self
  fn allocate(cache_key, color, font_system, swash) -> Option<(&[u8], cosmic_text::Placement)>
  fn trim()

### vector.rs
struct Pipeline { cache }
impl Pipeline
  fn new() -> Self
  fn viewport_dimensions(handle) -> Size<u32>
  fn draw(handle, color, bounds, opacity, pixels, transform, clip_mask)
  fn trim_cache()
struct Cache { trees, tree_hits, rasters, raster_hits, fontdb }
struct RasterKey { id, color, size }
impl Cache
  fn load(handle) -> Option<&usvg::Tree>
  fn viewport_dimensions(handle) -> Option<Size<u32>>
  fn draw(handle, color, size) -> Option<tiny_skia::PixmapRef<'_>>
  fn trim()
impl std::fmt::Debug for Cache

### window.rs

## rustploy-gui/vendor/iced_tiny_skia/src/window/

### compositor.rs
struct Compositor { context, settings }
struct Surface { window, clip_mask, layer_stack, background_color, max_age }
impl crate::graphics::Compositor for Compositor: with_backend, create_renderer, create_surface, configure_surface, information, present, screenshot
fn new(settings, display) -> Compositor
fn present(renderer, surface, viewport, background_color, on_pre_present) -> Result<(), compositor::SurfaceError>
fn screenshot(renderer, viewport, background_color) -> Vec<u8>
