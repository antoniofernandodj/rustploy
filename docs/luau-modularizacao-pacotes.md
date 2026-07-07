# Modularização da camada Luau em pacotes (`fmt/`, `handlers/`, `net/`)

## Contexto

A camada de rede/lógica do `rustploy-gui` (Parte 2 da migração RWP→HTTP/Luau,
ver `docs/migracao-http-luau.md`) começou concentrada em dois arquivos:
`app.luau` (1627 linhas — login, SSE, navegação, todas as mutações) e
`fmt.luau` (859 linhas — todos os formatadores/builders JSON). Isso ficou
grande demais para navegar/manter.

Ao quebrar em arquivos menores, a primeira tentativa foi organizar em
subpastas (`fmt/types.luau`, `handlers/stream.luau`, ...). Essa tentativa
esbarrou num conflito real entre o motor (glacier-ui) e o `luau-lsp` (o
type-checker), documentado abaixo, que forçou uma solução em duas etapas:
**corrigir o `require` do glacier-ui** e só então **reorganizar em pacotes**.

## O conflito descoberto

O `require()` do glacier-ui resolvia **sempre relativo ao diretório do script
de entrada** (as `roots`, calculadas *uma vez* na construção do componente, a
partir do `<script src="...">`/`<script>` do template) — não importava de
qual arquivo o `require` era chamado. Já o `luau-lsp` resolve `require(...)`
**relativo ao arquivo que faz a chamada** (estilo Node.js/Lua padrão).

Isso foi confirmado empiricamente com um repro isolado, fora do projeto:

```
/tmp/luautest/
  sub/
    b.luau   -- export type Foo = { n: number }; return {}
    d.luau   -- local B = require("sub/b"); type Foo = B.Foo; ...
```

Rodando `luau-lsp analyze sub/d.luau`, o checker reportou:

```
Unknown require: /tmp/luautest/sub/sub/b.lua
```

Ou seja: o luau-lsp tentou resolver `"sub/b"` relativo ao diretório do
**arquivo chamador** (`sub/`), produzindo `sub/sub/b.lua` — errado, porque a
string `"sub/b"` só fazia sentido relativa à RAIZ do projeto (onde o glacier
de fato resolveria, via as roots fixas do script de entrada). Nenhum dos dois
lados "sabia" do outro: o glacier ignorava o diretório do arquivo chamador; o
luau-lsp ignorava a raiz fixa do glacier.

Dois testes adicionais confirmaram os limites exatos:

- **Sem fallback para uma raiz**: `require("state")` (nome nu, sem prefixo) de
  dentro de `handlers/connection.luau`, esperando achar `state.luau` um nível
  acima, dava erro real no luau-lsp (`Unknown require:
  .../handlers/state.lua`) — o luau-lsp **não** tem nenhum fallback implícito
  para uma raiz fixa.
- **`../` explícito funciona nos dois lados**: `require("../state")` resolve
  corretamente no luau-lsp (navegação relativa clássica).
- **Irmão no mesmo diretório funciona nos dois lados**: `require("types")` de
  dentro de `fmt/util.luau`, esperando achar `fmt/types.luau` (arquivo
  irmão), resolve sem ressalvas.

Conclusão: para o motor **e** o checker concordarem 100% — sem gambiarra de
nomeação de arquivo, sem suprimir avisos —, dois requisitos:

1. O `require()` do próprio glacier-ui precisa passar a resolver relativo ao
   arquivo chamador (matching o luau-lsp).
2. Toda referência **entre pacotes diferentes** precisa usar `../` explícito
   (nenhum dos dois lados tem fallback implícito para uma raiz comum a partir
   de um módulo aninhado); só referências **dentro do mesmo pacote** (arquivo
   irmão) podem usar nome nu.

## Parte 1 — o fix no `require` do glacier-ui (v0.22.0)

Arquivo: `glacier-ui/src/luau/mod.rs`, função `install_module_system` (a
closure nativa registrada como o `require` global do interpretador) e
`resolve_module`/nova `normalize_modname`.

### Como o glacier descobre "quem está chamando `require` agora"

Usa `mlua::Lua::inspect_stack(1, ...)` — API do próprio `mlua` (existe desde
antes da 0.11, funciona com a feature `luau`) que inspeciona um nível da call
stack Lua a partir de dentro de uma função nativa Rust. Nível `1` = quem
chamou a função nativa atualmente em execução (confirmado pelo teste
`test_inspect_stack` do próprio `mlua`). O campo `Debug::source().source`
devolve a string exata passada em `.set_name(...)` no `load()` do chunk — para
um módulo carregado via `require`, isso é sempre `"@/caminho/absoluto.luau"`
(o glacier nomeia assim desde sempre). Basta remover o prefixo `@` e pegar o
diretório-pai.

O **script de nível superior** do componente (`<script>`/`<script src=...>`)
é carregado com o nome `<script:nome>` — **sem** o prefixo `@` — então
`inspect_stack` nunca acha um "diretório do chamador" para requires feitos
direto do corpo do script de entrada. Esse caso cai sempre no fallback de
sempre (as roots fixas), preservando 100% do comportamento documentado
anteriormente para esse cenário específico.

### As regras de resolução, para cada `require(modname)`

- **Prefixo `./`/`../` explícito**: busca **só** relativo ao diretório do
  arquivo chamador. Sem fallback — o mesmo contrato de um import relativo em
  qualquer linguagem: se não existir, é erro, não procura em outro lugar.
  Erro também se chamado sem um "arquivo chamador" (ex.: direto do script de
  nível superior).
- **Nome nu** (sem prefixo): tenta primeiro o diretório do arquivo chamador
  (irmão no mesmo pacote); se não achar, cai nas roots fixas de sempre
  (diretório do script de entrada + `lib/` + `GLACIER_LUAU_PATH`) — preserva
  bibliotecas "globais" alcançáveis de qualquer módulo aninhado, e é o único
  caminho disponível para requires feitos direto do script de nível superior.

### `normalize_modname` — por que dois estilos de separador coexistem

```rust
fn normalize_modname(modname: &str) -> String {
    let mut prefix = String::new();
    let mut rest = modname;
    loop {
        if let Some(r) = rest.strip_prefix("../") { prefix.push_str("../"); rest = r; }
        else if let Some(r) = rest.strip_prefix("./") { rest = r; }
        else { break; }
    }
    format!("{prefix}{}", rest.replace('.', "/"))
}
```

O glacier já suportava dois estilos de separador de pacote — ponto
(`require("net.http_client")`, estilo Lua clássico) e barra
(`require("net/api")`) — intercambiáveis, porque a implementação antiga fazia
um `modname.replace('.', "/")` cego. Isso quebraria com um prefixo `../`
literal (os pontos de `..` também seriam trocados por barra, virando `//`).
`normalize_modname` extrai o prefixo de navegação literal primeiro, e só
aplica a troca ponto→barra no restante — os dois estilos nunca colidem,
porque nenhum nome de pacote legítimo começa com `.`.

### Cache por caminho resolvido, não pela string pedida

Antes, o cache de módulos carregados era uma tabela `modname_string → valor`.
Com resolução relativa ao chamador, a **mesma string** (ex.: `"types"`) pode
agora resolver a **arquivos diferentes** dependendo de quem chama (ex.: dois
pacotes distintos, cada um com seu próprio `types.luau`). Manter a chave do
cache como a string pedida colidiria os dois. A chave do cache passou a ser o
**caminho absoluto canonicalizado** do arquivo resolvido.

### Testes novos (glacier-ui, `src/luau/mod.rs`)

- `require_bare_entre_irmaos_em_modulo_aninhado_resolve_pelo_proprio_diretorio`
  — um módulo NÃO-topo faz `require("b")` esperando achar um irmão no MESMO
  diretório, não na raiz. É o teste que teria falhado com o código antigo.
- `require_com_prefixo_dotdot_sobe_um_nivel` — `require("../shared")` de
  dentro de um pacote aninhado, esperando achar um arquivo um nível acima.
- `require_de_mesmo_nome_em_pacotes_diferentes_nao_colide_no_cache` — dois
  módulos, em pacotes diferentes, fazem `require("x")` esperando arquivos
  DIFERENTES; prova que a chave do cache por caminho resolvido não os
  confunde.

Todos os testes pré-existentes (`resolve_module_acha_arquivo_e_init`,
`require_de_script_externo_resolve_relativo_ao_script`, o exemplo
`imports_luau`) continuam passando sem alteração — a mudança é aditiva e
retrocompatível.

## Parte 2 — reorganização do rustploy em pacotes

Com o `require` corrigido, a convenção de escrita para cada `require(...)` na
camada Luau do rustploy-gui é:

- **Irmão no mesmo pacote** (mesmo diretório): nome nu —
  `require("stream")`, `require("types")`.
- **Pacote diferente** (inclusive os módulos "raiz" `state`/`helpers`/`fmt`/
  `net/api`, referenciados de dentro de `handlers/` ou `fmt/`): `../`
  explícito — `require("../state")`.
- **Do script de entrada** (`app.luau`, que fica na raiz de
  `views/scripts/`): nome nu de caminho completo — `require("handlers/connection")`,
  `require("fmt/time")` — resolve pelo fallback de roots fixas (idêntico ao
  comportamento de sempre para o script de nível superior).

### Estrutura final

```
crates/rustploy-gui/views/scripts/
  app.luau              # entrada — só faz require de cada handlers/*
  state.luau             # estado mutável compartilhado (tabela única)
  helpers.luau            # utilitários puros (trim, notify_ok/err, ...)
  glacier.d.luau          # tipos p/ luau-lsp (nunca é require'd em runtime)
  net/
    api.luau              # cliente HTTP/JSON da API do daemon
  fmt/                    # (fmt.luau, a fachada, fica ao lado — padrão
    types.luau            #  "mod.rs ao lado do dir" do próprio Rust)
    time.luau
    util.luau
    dashboard.luau
    service_detail.luau
    git.luau
  fmt.luau                # fachada: fmt.foo(...) reexporta fmt/*.luau
  handlers/
    connection.luau        # login/logout, init(), settings buscadas na conexão
    stream.luau             # consumidor do SSE (snapshot + eventos bus)
    nav.luau                 # navegação da sidebar/tabs, busca do topbar
    services.luau             # detalhe do serviço: fetch, mutações, ciclo de vida
    projects.luau              # grade de projetos, env de projeto, stop_all
    wizard.luau                 # wizard "Novo serviço"
    settings.luau                # Settings (Web Server) + Settings → Git
    docker.luau                   # limpeza de imagens/volumes/redes sem uso
```

`fmt.luau` é uma **fachada**: só reexporta os campos de cada submódulo
`fmt/*.luau` sob uma única tabela (`fmt.services(...)`, `fmt.fmt_bytes(...)`,
...), então nenhum handler precisou mudar seus call sites — só as strings de
`require` internas de cada arquivo movido.

### Regra de ouro para novas funções

Só funções **literalmente referenciadas** por `onClick`/`onChange`/`onSubmit`/
`onToggle`/`onReorder` em XML, pelos `confirm_action` dinâmicos construídos em
runtime (`"do_delete_project:" .. id`), pelos callbacks nomeados do `sse()`
(`on_message`/`on_error`/`on_close`), ou o hook de lifecycle `init()`,
**precisam** ser funções globais (`function nome(...)`, sem `local`) — é o
único jeito do motor achá-las (ele busca por STRING em
`self.luau.globals()`). Toda lógica interna cross-módulo é função **local**
exportada numa tabela de retorno, consumida via
`local X = require("../pacote"); X.fn()` — grafo de dependência explícito,
sem globals cross-arquivo implícitos.

### Auditoria de segurança usada na verificação

Um script Python (rodado antes e depois da reorganização) extrai toda
ação-base de `onClick`/`onChange`/`onSubmit`/`onToggle`/`onReorder`/`action=`
dos `.xml` e cruza com todo `^function nome` definido nos `.luau` de
`handlers/` — garante que nenhum handler ficou órfão numa reorganização
futura:

```python
import re, glob

attr = re.compile(r'(?:on_?click|on_?change|on_?submit|on_?toggle|on_?reorder|on_?press|action)\s*=\s*"([^"]+)"', re.I)
actions = set()
for f in glob.glob('views/**/*.xml', recursive=True):
    for m in attr.finditer(open(f).read()):
        actions.add(m.group(1).strip())

def basename(a):
    a = a.strip()
    if a.startswith('{') and a.endswith('}'):
        return None
    return a.split(':', 1)[0].split('{', 1)[0].strip()

bases = {basename(a) for a in actions if basename(a)}

defined = set()
for f in glob.glob('views/scripts/handlers/*.luau') + ['views/scripts/app.luau']:
    defined |= set(re.findall(r'^function\s+([A-Za-z_]\w*)', open(f).read(), re.M))

builtins = {'clipboard', 'open', 'window', 'nav'}
missing = [b for b in sorted(bases) if b not in defined and b not in builtins]
print("AÇÕES SEM HANDLER:", missing)  # deve ser []
```

## Verificação (rodada e verde antes de commitar)

1. **glacier-ui**: `cargo test` — 74 testes de lib + 42 de integração, todos
   verdes (incluindo os 3 novos); exemplo `imports_luau` compila.
2. **`cargo publish --dry-run` / `cargo publish`** — glacier-ui 0.22.0 no
   crates.io.
3. **luau-lsp, genuinamente limpo** (não suprimido por coincidência de
   diretório único):
   ```
   luau-lsp analyze --base-luaurc=.luaurc \
     --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau \
     <os 19 arquivos .luau>
   ```
   → 0 erros, 0 warnings.
4. **Runtime real (mlua)**: `cargo test -p rustploy-gui --test
   templates_render` — prova que a resolução de módulos funciona em produção
   (não só no checker), exercitando `init()` e todas as telas.
5. **Auditoria de ações** (script acima) — 0 ações órfãs.
6. `cargo build -p rustploy-gui` (com `glacier-ui = "0.22.0"` no
   `Cargo.toml`) — 0 erros.

## Setup do `luau-lsp` (CLI + editor)

### Instalação do binário

```bash
curl -L https://github.com/JohnnyMorganz/luau-lsp/releases/latest/download/luau-lsp-linux-x86_64.zip -o /tmp/luau-lsp.zip
unzip -o /tmp/luau-lsp.zip -d ~/.local/bin/
luau-lsp --version   # ex.: 1.68.1
```

(Troque `linux-x86_64` pelo asset certo em
[releases](https://github.com/JohnnyMorganz/luau-lsp/releases/latest) para
macOS/Windows. `~/.local/bin` só precisa estar no `PATH`.)

### Extensão do editor (VS Code)

O `.vscode/settings.json` do repo (ver abaixo) só tem efeito se a extensão
estiver instalada — ela é quem lê essas chaves e fala com o binário
`luau-lsp`. Publisher/nome: **`johnnymorganz.luau-lsp`** ("Luau Language
Server", mesmo autor do `luau-lsp`). Instalar pelo marketplace do VS Code
(`Ctrl+P` → `ext install johnnymorganz.luau-lsp`) ou:

```bash
code --install-extension johnnymorganz.luau-lsp
```

A extensão baixa/gerencia seu PRÓPRIO binário `luau-lsp` por padrão (não
precisa ser o mesmo `~/.local/bin/luau-lsp` da seção anterior — são usos
independentes: um pela CLI/CI, outro pelo editor). Depois de instalar,
recarregar a janela (`Ctrl+Shift+P` → "Reload Window") para os settings do
repo (`.luaurc` + `.vscode/settings.json`) serem lidos.

### Validação via CLI (o que esta investigação usou)

```bash
luau-lsp analyze --base-luaurc=.luaurc \
  --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau \
  <arquivo(s) .luau a checar>
```

Rodar isso (ou passar TODOS os `.luau` de uma vez) antes de considerar uma
mudança na camada Luau pronta — pega em segundos um erro de caminho de
`require` ou de tipo que só apareceria em runtime (ou nem apareceria, se o
módulo com bug nunca for de fato exercitado pelos testes automatizados). Não
é substituto de rodar o app de verdade: garante que TIPOS e CAMINHOS de
módulo batem, mas quem prova que o `require` realmente carrega e executa em
produção é o `mlua` em runtime (ver
`crates/rustploy-gui/tests/templates_render.rs`).

### As três peças de configuração e o que cada uma faz

- **`crates/rustploy-gui/views/scripts/glacier.d.luau`** — o *definitions
  file*: declara os globais que o motor glacier-ui injeta em runtime e que
  não existem como `require`/arquivo (`ctx: {[string]:string}`, `value`,
  `fetch`/`sse`/`websocket`→`FetchResult`/`StreamHandle`, `toast`, `confirm`,
  `json.decode`/`encode`/`array`). Usa a sintaxe especial `declare
  X: T`/`declare function foo()`, que só é válida quando o arquivo é
  carregado NO MODO DEFINITIONS (`--definitions=` na CLI, ou
  `luau-lsp.types.definitionFiles` no editor — ver abaixo). Fora desse modo
  cada `declare` é um erro de sintaxe (ver "Armadilha" logo abaixo).
- **`.luaurc`** (raiz do repo) — config do `luau-lsp` em si:
  ```json
  {
      "languageMode": "strict",
      "globals": ["ctx", "value", "fetch", "websocket", "sse", "require", "json", "confirm", "toast"]
  }
  ```
  `languageMode: strict` liga a checagem de tipos completa por padrão (sem
  precisar de `--!strict` redundante em cada config); `globals` é a lista de
  nomes que o checker deve aceitar como ambiente global mesmo sem
  `declare` — cobre os mesmos nomes do `glacier.d.luau` (o `.luaurc` é uma
  segurança adicional/redundante; o `glacier.d.luau` é a fonte de tipos
  precisos).
- **`.vscode/settings.json`** (raiz do repo) — liga a extensão `luau-lsp` do
  VS Code ao mesmo setup que a CLI já usava:
  ```json
  {
      "luau-lsp.platform.type": "standard",
      "luau-lsp.types.definitionFiles": [
          "crates/rustploy-gui/views/scripts/glacier.d.luau"
      ]
  }
  ```
  `platform.type: standard` evita o modo Roblox padrão da extensão (não
  usamos Roblox aqui — sem isso a CLI já avisava: `WARNING: --platform is
  set to 'roblox'`). `types.definitionFiles` é o equivalente, para a
  extensão, do `--definitions=` da CLI.

### Armadilha real: sem `.vscode/settings.json`, o editor acusa ~40 erros falsos em `glacier.d.luau`

Sem a config acima, a extensão do VS Code analisa `glacier.d.luau` como um
script COMUM (não como definitions file) — e `declare ctx: {...}` não é uma
statement válida em Luau comum. Isso cascateia em dezenas de diagnósticos
(`SyntaxError: Incomplete statement`, depois `SameLineStatement`,
`FunctionUnused`, `BuiltinGlobalWrite: Built-in global 'fetch' is
overwritten here`, ...) — todos FALSOS, o arquivo está correto. Reproduzido e
confirmado (2026-07-07):

```bash
# sem --definitions=: ~40 erros (reproduz o que o editor mostrava)
luau-lsp analyze --base-luaurc=.luaurc crates/rustploy-gui/views/scripts/glacier.d.luau

# com --definitions=: 0 erros
luau-lsp analyze --base-luaurc=.luaurc \
  --definitions=crates/rustploy-gui/views/scripts/glacier.d.luau \
  crates/rustploy-gui/views/scripts/app.luau
```

O `.vscode/settings.json` documentado acima resolve isso (recarregar a
janela do VS Code / reiniciar o Luau Language Server depois de criá-lo).

## Outras lições para o futuro

- Ao adicionar um módulo `.luau` novo em qualquer subpasta do rustploy-gui,
  seguir a convenção de `require` (nome nu = irmão; `../` = pacote pai) e
  rodar a validação da CLI (seção acima) antes de considerar pronto.
