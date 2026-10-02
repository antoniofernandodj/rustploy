# Índice: ferramentas do repositório

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## tools/indexer/src/

### commands.rs — `comandos.md`: uma linha por variante de `Command` ligando as três pontas de uma feature: o handler …
struct Variant { name, group, doc }
fn render(root, files) -> Result<String, String> — Falha (sem gerar nada) se não achar o `enum Command` ou nenhum handler: uma tabela vazia seria pior …
fn variants(root) -> Result<Vec<Variant>, String> — Variantes na ordem de declaração, com o grupo dado pelo último comentário `// Grupo` visto acima del…
fn handlers(root) -> Result<BTreeMap<String, String>, String> — Variante → `arquivo` (ou `arquivo::fn`) do handler, dos braços de todo `match` em `routes.rs` cujo c…
fn callers(root, files, prefix, ext, comment, pattern) -> BTreeMap<String, BTreeSet<String>> — Variante → arquivos (relativos a `prefix`) que a mencionam como Command.
const PROTOCOL, ROUTES, GUI_SCRIPTS, WEBUI, AGENT

### main.rs — Gera `docs/indice/`: um mapa de arquivos e símbolos pensado para um agente achar "onde está a respon…
const COMMANDS_MD — Índice transversal (não é área de prefixo): ver `commands.rs`.
const AREAS — Áreas do índice de símbolos: (arquivo gerado, título, prefixos cobertos).
const COLLAPSE — Diretórios de dados que viram uma linha só no INDEX.md.
const NAMES_ONLY — Diretórios que o INDEX.md lista só por nome, numa linha: a descrição de cada arquivo repetiria um ín…
const COMMON_TRAITS — Traits cujos métodos são ditados pela própria trait: listar os métodos só gastaria token.
fn main() -> ExitCode
fn run() -> Result<(), Vec<String>> — Primeiro gera tudo em memória e checa; só grava se nada falhou.
fn repo_root() -> Result<PathBuf, String> — Raiz do **agregador** (onde moram `docs/` e os submodules): o toplevel git de onde o indexador roda.
fn submodule_dirs(root) -> Vec<String> — Diretórios dos submodules, lidos do `.gitmodules` do agregador.
fn ls_files(dir, prefix) -> Result<Vec<String>, String> — `git ls-files` (versionados + novos não ignorados) de um repo, com `prefix/`.
fn tracked_files(root) -> Result<Vec<String>, String> — Arquivos do agregador **e** dos submodules (com o diretório do submodule como prefixo), para indexar…
fn ext_of(path) -> &str
fn is_indexed(path) -> bool — Arquivos cujos símbolos entram nos índices de área.
fn area_of(path) -> Option<usize>
fn split_dir(path) -> (&str, &str)
fn render_index(root, files) -> String
fn describe_file(root, path) -> Option<String> — Uma linha de descrição por arquivo, só de fontes baratas e confiáveis.
fn render_area(root, title, paths, errors) -> String — Um arquivo que não dá para ler ou parsear vai para `errors` (e o índice não é gravado), em vez de vi…
struct Renderer { out, tests }
impl Renderer
  fn line(indent, body, attrs)
  fn items(items, indent)
  fn const_like(indent, name, attrs, rest) — Constantes documentadas ganham linha própria; as outras viram uma lista.
  fn impl_block(im, indent, common)
fn count_tests(items) -> usize
fn is_test(attrs) -> bool — `#[test]`, `#[tokio::test]` e afins.
fn is_cfg_test(attrs) -> bool
fn signature(sig) -> String — `nome(a, b) -> Ret`: só os nomes dos parâmetros (os tipos custam caro e raramente ajudam a achar) e …
fn tokens(t) -> String — Tokens → texto compacto (`Result < Vec < T > >` → `Result<Vec<T>>`).
fn short(s, max) -> String
fn doc_summary(attrs) -> Option<String> — Primeira frase da doc (`///` ou `//!`), limitada, numa linha só.
fn summarize(lines) -> Option<String> — Primeira frase do primeiro parágrafo de um comentário, limitada, numa linha só.
const OUT_DIR, BINARY_EXT, RUST_NOTATION

### script.rs — Índice dos arquivos que não são Rust: scripts Luau e JS, templates `.gv`, `index.html` da webui e fo…
fn notation(ext) -> Option<&'static str> — Uma linha de notação por tipo de arquivo, para o cabeçalho da área.
fn re(pattern) -> Regex
fn describe(path, text) -> Option<String> — Primeira frase do comentário que abre o arquivo, sem o prefixo `nome.ext — ` que vários arquivos rep…
fn line_comment_header(text, marker, skip) -> Vec<String> — Linhas do primeiro bloco de comentários de linha (`--`, `//`) no topo.
fn block_comment_header(text, open, close) -> Vec<String> — Conteúdo do primeiro comentário de bloco (`/* */`, `<!-- -->`) do arquivo, desde que só haja preâmbu…
fn render(path, text) -> String
struct PendingDoc(Vec<String>) — Junta os comentários consecutivos logo acima de uma definição.
impl PendingDoc
  fn feed(line, markers) -> bool
  fn take() -> Option<String>
fn with_doc(out, body, doc)
fn param_names(params) -> String — `a: string, b: number?` → `a, b`; `kind = "info"` → `kind`.
fn render_luau(text) -> String
fn render_js(text) -> String
fn render_gv(text) -> String
fn render_html(text) -> String
fn strip_blocks(text, open, close) -> String
const EXTENSIONS, LUAU_FN, LUAU_ASSIGN_FN, LUAU_TYPE, JS_CONTAINER, JS_TOP_FN, JS_TOP_ARROW, JS_METHOD, JS_GETTER, JS_KEYWORDS, GV_ROOT, GV_PROP, GV_IMPORT, GV_SCRIPT, GV_VIEW, GV_HANDLER, HTML_SECTION, HTML_XDATA, HTML_EVENT, CALL
