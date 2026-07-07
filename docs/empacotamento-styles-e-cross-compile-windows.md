# Empacotamento do rustploy-gui: mover styles/ e cross-compile Windows (cargo-xwin)

Registro de uma sessão de trabalho com duas partes encadeadas: (1) mover
`crates/rustploy-gui/styles/` para dentro de `views/styles/` e garantir que os
dois empacotamentos (`.deb` e `.zip` Windows) continuassem incluindo os
arquivos certos, e (2) o problema de toolchain que apareceu ao tentar validar
o build Windows via `cargo-xwin` nesta máquina.

---

## 1. Contexto: por que `styles/` virou `views/styles/`

`app.gss` é a folha `.gss` **compartilhada** por praticamente todo template do
GUI (`card`, `btn_primary`, `title`, `subtitle`, `muted`, `field_input`,
`tab`, `kv_row`, ...). Ela é linkada a partir de `app.xml` via
`<link rel="stylesheet" href="crates/rustploy-gui/views/styles/app.gss" />`.

Isso só passou a funcionar corretamente depois de uma mudança no motor
`glacier-ui` (0.22 → 0.23): antes, tanto um `<link rel="stylesheet">` quanto
um `<style>` inline eram **escopados ao componente que os declarava** — como
`app.xml` importa `Login`/`Shell`/etc. como componentes próprios, o escopo
nunca alcançava essas classes lá, quebrando o app inteiro fora de `app.xml`
sempre que a folha era linkada via XML em vez de `motor.load_stylesheet()` em
Rust. A partir da 0.23, `<link rel="stylesheet">` e `<style>` (sem atributo)
passaram a ser **globais por padrão**; escopo agora é opt-in via
`<style scoped="true">`. Com isso, o arquivo `app.gss` pôde ser movido para
dentro de `views/` (onde já moram os outros templates/scripts) sem quebrar
nada, e o `motor.load_stylesheet()` duplicado em Rust (`app/mod.rs`) pôde
sair — `app.xml` já carrega a folha sozinho.

## 2. O que quebrou no empacotamento depois da mudança de path

Depois do `git mv styles/ → views/styles/`, três lugares que ainda apontavam
pro caminho antigo (`crates/rustploy-gui/styles/`) ficaram quebrados ou
redundantes:

| Lugar | Problema |
|---|---|
| `crates/rustploy-gui/Cargo.toml` (`[package.metadata.deb] assets`) | tinha uma entrada `["styles/**/*", ...]` separada, apontando pro diretório que não existe mais |
| `Makefile` (`rustploy-gui-windows-dist`) | `cp -r crates/rustploy-gui/styles ...` — `styles/` não existe mais, o `cp` falharia (source inexistente) |
| `.github/workflows/release.yml` (job `build-gui-windows`) | mesmo `cp -r crates/rustploy-gui/styles ...` que o Makefile (os dois têm que ficar em sincronia manualmente, não há glob/manifest tool nesse job) |

### A correção

- **`.deb`**: o glob `["views/**/*", ...]` já existente cobre
  `views/styles/*.gss`/`*.json` automaticamente — bastou **remover** a entrada
  `styles/**/*`, que era redundante e agora aponta pra um diretório inexistente.
- **Windows (`Makefile` + `release.yml`)**: `views/` já é copiada **inteira**
  (é o mesmo motivo pelo qual um bug anterior — commit `a727927` — tinha
  corrigido o esquecimento de `views/scripts/`: copiar por sub-pasta faz esse
  target esquecer um pacote novo silenciosamente). Bastou **remover** o
  `cp -r crates/rustploy-gui/styles ...` de ambos.
- Adicionado, nos dois lugares (Makefile e workflow), um
  `test -f .../views/styles/app.gss || (echo ERRO ... && exit 1)` — mesmo
  espírito do check já existente pra `views/scripts/app.luau` — pra pegar essa
  classe de regressão de empacotamento cedo da próxima vez que algum diretório
  for renomeado/movido.
- Referências de documentação (`assets.rs`, `CLAUDE.md`) também atualizadas
  pro novo caminho.

Validação: `make -n rustploy-gui-windows-dist` (dry-run) conferido
sintaticamente; `cargo check -p rustploy-gui`, `cargo test -p rustploy-gui
--test templates_render` e os testes do `glacier-ui` (`cargo test`, 119
passed) todos verdes antes do commit/push.

---

## 3. O que deu errado ao tentar validar o build Windows nesta máquina

Rodar `make rustploy-gui-windows-dist` (que faz `cargo xwin build --release
-p rustploy-gui --target x86_64-pc-windows-msvc`) falhava assim, **sem
relação nenhuma com a mudança de `styles/`** — é um problema de toolchain
local para cross-compilar C++ (o `mlua-sys`/`luau0-src`, dependência da
camada Luau, embute o parser do Luau em C++ e precisa compilar isso pro
target `x86_64-pc-windows-msvc`):

```
error: static assertion failed: error STL1000: Unexpected compiler version, expected Clang 19.0.0 or newer.
```

### Causa

`cargo-xwin` baixa (via `xwin`) os headers/libs da MSVC CRT/SDK e usa
`clang-cl` como compilador C/C++ cross para esse target. Os headers da MSVC
STL que o `xwin` baixou (`~/.cache/cargo-xwin/xwin/crt/include/yvals_core.h`)
fazem um `static_assert` exigindo Clang **19+** — e o `clang` default do
Ubuntu nesta máquina era o 18 (`clang-18`, via `apt install clang`).

O detalhe que custou mais tempo pra descobrir: `cargo-xwin` **não** procura
`clang-cl` no `PATH` do sistema. A cada `cargo xwin build`, ele recria (sempre,
incondicionalmente) o symlink `~/.cache/cargo-xwin/clang-cl` apontando
**hardcoded** pra `/usr/bin/clang` (não pra uma versão específica, nem via
`which clang`). Então:

- Instalar `clang-19` via apt **não resolveu sozinho** — o pacote não cria um
  `clang-cl-19`, e mesmo criando manualmente um symlink alternativo
  (`ln -sf /usr/bin/clang-19 ~/.cache/cargo-xwin/clang-cl`), o próximo
  `cargo xwin build` **sobrescreve esse symlink de volta pro clang 18**, porque
  ele sempre recria apontando pra `/usr/bin/clang` — e é `/usr/bin/clang` que
  precisa mudar, não o symlink dentro do cache do xwin.

### O que resolveu

Registrar `clang` no `update-alternatives` do sistema e apontar o default pro
19 — assim `/usr/bin/clang` (que é o que `cargo-xwin` sempre symlinka) passa a
resolver pro clang 19, e a correção sobrevive a qualquer recriação do symlink
pelo cargo-xwin:

```bash
sudo apt install -y clang-19

sudo update-alternatives --install /usr/bin/clang clang /usr/lib/llvm-19/bin/clang 190
sudo update-alternatives --install /usr/bin/clang clang /usr/lib/llvm-18/bin/clang 180
sudo update-alternatives --install /usr/bin/clang clang /usr/lib/llvm-17/bin/clang 170
sudo update-alternatives --set clang /usr/lib/llvm-19/bin/clang
```

Depois disso, `make rustploy-gui-windows-dist` (`cargo xwin build ...`)
compilou o `mlua-sys`/`luau0-src` sem erro.

### O que **não** funcionou (registrado pra não tentar de novo)

- Symlinkar `~/.cache/cargo-xwin/clang-cl` diretamente pra `clang-19` — é
  sobrescrito no próximo `cargo xwin build`.
- `clang-19` sozinho, sem `update-alternatives`, não é suficiente: o pacote
  Ubuntu não gera um binário `clang-cl-19` (só o `clang-17` gera
  `clang-cl-17`); é o binário `clang` puro (que já entende o modo
  `clang-cl` via nome do processo/symlink) que precisa ser o 19 por trás de
  `/usr/bin/clang`.

### Reversão, se precisar

```bash
sudo update-alternatives --config clang
```

---

## Resumo / checklist para a próxima vez que mexer em `views/` ou em assets do GUI

- [ ] Se renomear/mover um diretório de asset (`views/`, `styles/`, `scripts/`,
      `assets/icons/`), procure por `cp -r crates/rustploy-gui/<nome-antigo>`
      no `Makefile` **e** em `.github/workflows/release.yml` — não há glob
      automático nesse job, tem que atualizar os dois à mão.
- [ ] O job `.deb` (`cargo-deb`) usa glob (`views/**/*`) — normalmente não
      precisa de mudança quando algo já está dentro de `views/`.
- [ ] Cross-compile Windows local via `cargo-xwin` precisa de `clang` (o
      binário puro, não uma variante `-cl`) na versão exigida pelos headers
      que o `xwin` baixa — hoje, 19+. Confirme com
      `update-alternatives --display clang`.
