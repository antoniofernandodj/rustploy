--!strict
-- app.luau — ponto de entrada do `<script>` de app.gv. Não define nenhuma
-- lógica própria: apenas carrega (via `require`) cada pacote de
-- `handlers/`, para que suas funções globais (referenciadas por
-- on_click/on_change/on_submit nos templates, e pelos callbacks nomeados do
-- sse()) fiquem registradas antes de qualquer ação disparar. Cada handler
-- cuida de UM domínio da UI; o estado mutável compartilhado vive em
-- `state.luau`; utilitários puros em `helpers.luau`/`fmt.luau` (fachada do
-- pacote `fmt/`). Ver `state.luau` para a convenção de `require` entre
-- pacotes (nomes nu = irmão no mesmo diretório; `../` = pacote pai).

require("handlers/connection")
require("handlers/stream")
require("handlers/nav")
require("handlers/services")
require("handlers/projects")
require("handlers/wizard")
require("handlers/settings")
require("handlers/docker")
require("handlers/registry")
require("handlers/jobs")
