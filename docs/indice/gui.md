# Índice: rustploy-gui (Rust)

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> `.gvb`: `<app>`/`<screen>`/`<component>`, `props`, `imports` (`link(rel = import)`), `script`, `telas` (as `screen(name = …)` do app), `views` (valores de `@view ==`), `abas` (valores de `@tab`/`@*_tab`) e `handlers` (`on_*` e `action`).
> Luau: `function x(a)` = global (handler que o `.gv` chama pelo nome), `local x(a)` = privada, `function M.x(a)` = exportada pelo módulo.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-gui/

### build.rs — Build script: logos dos blueprints para o release e recursos do `.exe` no Windows.
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
fn install_desktop_entry()
fn is_cargo_install() -> bool
fn write_desktop_entry() -> Result<(), String>
fn exec_com(desktop, exe) -> String — Mesmo escape de `src/desktop.rs::exec_com` (duplicado: o build script não enxerga o crate).
fn write_file(path, bytes) -> Result<(), String>
fn copy_tree(src, dst) -> Result<(), String>

## rustploy-gui/src/

### assets.rs — Runtime asset location.
const MARKER — A file that must exist under any valid asset base — used as the probe.
const SYSTEM_PREFIX — System-wide install prefix used by the Debian package (see the `deb` metadata in `Cargo.toml`).
fn locate_and_chdir() — Finds the asset base directory and `chdir`s into it so all the CWD-relative asset paths resolve.
fn find_base() -> Option<PathBuf>
fn has_marker(base) -> bool

### desktop.rs — `rustploy-gui --install-desktop`: integração com o desktop para quem instalou por `cargo install` (o…
const ICONS — `(subdiretório hicolor, bytes)` — espelha `packaging/icons/hicolor/`.
fn handle_args() -> Option<i32> — Trata `--install-desktop`.
fn install() -> Result<(), String>
fn install() -> Result<(), String>
fn data_home() -> Result<PathBuf, String>
fn exec_com(desktop, exe) -> String — Troca a linha `Exec=` pelo caminho absoluto deste executável (entre aspas, como exige a spec do Desk…
fn write(path, bytes) -> Result<(), String>
const DESKTOP, ICON_SVG
(2 testes)

### embedded.rs — Assets embutidos no binário — modo standalone (só em builds de release).
const VIEWS — `views/`: templates `.gvb`, estilos `styles/*.gss`, `styles/theme.json` e os scripts Luau em `script…
const ICONS — `assets/icons/`: ícones SVG referenciados por `<svg src="assets/icons/…">`.
const FONTS — `assets/fonts/`: as fontes que o `app(...)` declara com `font(src = …)`.
const BLUEPRINTS — Logos dos blueprints, referenciados via o `{logo}` data-driven do catálogo do daemon (`assets/bluepr…
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

## rustploy-gui/src/app/

### mod.rs — Rustploy (glacier-ui) — desktop client whose UI is described in XML templates and rendered by the pu…
fn run() -> iced::Result — Sobe o daemon multi-janela e roda o loop do iced até a última janela fechar.

## rustploy-gui/tests/

### access_key.rs — O `access_key.luau` (token de conexão do login) rodando no motor de verdade.
fn boot() -> GlacierUI
(2 testes)

### busy.rs — busy.luau no motor de verdade: o botão libera no toast da própria ação, não só quando o handler inte…
(1 testes)

### format_service_detail.rs — O `format/service_detail.luau` (`compose_host` e `internal_url`) rodando no motor de verdade.
fn boot() -> GlacierUI
(2 testes)

### format_time.rs — O `format/time.luau` rodando no motor de verdade.
fn boot() -> GlacierUI
fn offset_local_segundos() -> i64 — Offset local em segundos, perguntado ao sistema — a mesma fonte que o `localtime` do Luau consulta.
fn shifted_time(hms, offset) -> String — `HH:MM:SS` + offset, com a virada de dia descartada (só as horas importam).
(3 testes)

### login_saved_servers.rs — Tela de login com servidores lembrados: o `<select>` mostra `host:porta`, o campo recebe o token de …
const SAVED
(1 testes)

### templates_render.rs — Headless validation: every template parses, every screen/tab evaluates and builds an iced element tr…
fn boot() -> GlacierUI — Boots the engine the way `main.rs` does, but from the workspace root so the workspace-relative templ…
fn textos_avaliados(n) -> Vec<String> — Todo texto de uma árvore avaliada, em ordem.
fn cd_ws_root() — Cd's to the workspace root (idempotent — safe alongside `boot`).
fn comments_outside(src) -> String — Remove os comentários (`//` e `/* … */`) para que "a primeira tag" seja a primeira tag de verdade: t…
(18 testes)

## rustploy-gui/tests/fixtures/

### access_key_fixture.gvb — Fixture do teste access_key.rs: tela mínima que roda o access_key.luau e deixa os resultados no cont…
<screen "fixture">
script: access_key_fixture.luau

### access_key_fixture.luau — Fixture do teste `access_key.rs` (nome diferente do módulo, senão o `require` resolve para ela mesma…
function init()

### compose_host.gvb — Fixture do teste format_service_detail.rs: tela mínima que roda o format/service_detail.luau e exibe…
<screen "fixture">
script: compose_host.luau

### compose_host.luau — Fixture do teste `format_service_detail.rs`: exercita `compose_host` e `internal_url` de `format/ser…
function init()

### saved_action.gvb — Fixture do teste busy.rs: um botão cuja ação toasta e depois ainda faz outro fetch (o "reler o servi…
<screen "fixture">
script: saved_action.luau
handlers: salvar

### saved_action.luau — Fixture do teste `busy.rs`.
function init() — Fixture do teste `busy.rs`.
function salvar()

### time.gvb — Fixture do teste format_time.rs: tela mínima que roda o format/time.luau e exibe o resultado.
<screen "fixture">
script: time.luau

### time.luau — Fixture do teste `format_time.rs`: exercita o `format/time.luau` de verdade, através do motor, e dei…
function init()

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
