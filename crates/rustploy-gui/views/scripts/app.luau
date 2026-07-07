--!strict
-- app.luau — ponto de entrada do `<script>` de app.xml. Não define nenhuma
-- lógica própria: apenas carrega (via `require`) todos os módulos de handlers,
-- para que suas funções globais (referenciadas por onClick/onChange/onSubmit
-- nos templates, e pelos callbacks nomeados do sse()) fiquem registradas antes
-- de qualquer ação disparar. Cada handler cuida de UM domínio da UI; o estado
-- mutável compartilhado vive em `state.luau`; utilitários puros em
-- `helpers.luau`/`fmt.luau`. Ver o comentário em `state.luau` sobre por que os
-- módulos são todos irmãos (nomes `escopo_sufixo.luau`) em vez de
-- subdiretórios aninhados.

require("handlers_connection")
require("handlers_stream")
require("handlers_nav")
require("handlers_services")
require("handlers_projects")
require("handlers_wizard")
require("handlers_settings")
require("handlers_docker")
