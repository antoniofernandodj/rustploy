# Inventário de paridade — GUI iced × webui

Levantamento da Fase 0 de `docs/plano-convergencia-templates-gui-webui.md`:
o que existe de um lado e não do outro, tela a tela. Base para decidir o que
é divergência a corrigir (2.B/2.C do plano) e o que é cromo de plataforma
(seção 5 do plano, fica divergente de propósito).

## Telas/abas — presença

| Tela / aba | iced (`views/`) | webui (`webui/`) | Nota |
|---|---|---|---|
| Login | `login.gv` | `index.html` (`screen==='login'`) | paridade |
| Deployments | `shell.gv` | `screens/dashboard.js` | paridade |
| Projects (grid) | `shell.gv` | `screens/projects.js` | webui sem janela separada de criação — form inline (5.) |
| Projeto aberto (Serviços/Env/Secrets/Jobs) | view=`project_services` | view=`project` | **nome de view diverge** (mecânico, não funcional) |
| Deploy Engine | `home.gv` | `screens/deploy_engine.js` | paridade |
| Monitoring | `home.gv` | `screens/monitoring.js` | paridade |
| Ingress | `home.gv` | `screens/ingress.js` | paridade |
| Docker — Containers/Images/Volumes/Networks/Registry | `home.gv` (`docker_tab`) | `screens/docker.js` (`store.dockerTab`) | paridade (webui já tem as 5 sub-abas) |
| Schedules | `home.gv` | `screens/schedules.js` | paridade |
| Settings — Web/Git/IaC/Maintenance | `home.gv` (`settings_tab`) | `index.html` (`store.settingsTab`) | paridade nas 4 sub-abas |
| Serviço — General/Connection/Environment/Domains/Deployments/Healthcheck/Logs/Advanced | `service.gv` (`tab`) | `index.html` (`store.serviceTab`) | **General era só-leitura na webui — corrigido nesta sessão** (ver abaixo) |
| Support | `shell.gv` (`nav_support`) | — | **só no iced** — divergência de propósito, não implementada na webui |
| Janela "Novo projeto" | `new_project_form.gv` (janela separada) | form inline em `screens/projects.js` | 5. cromo de plataforma |
| Janela "Novo job" | `new_job_window.gv` (janela separada) | modal no store (`njob*`) | 5. cromo de plataforma |
| Janela "Novo serviço" (wizard) | `new_service_window.gv` + `new_service.gv` | `screens/new_service.js` inline | 5. cromo de plataforma |
| Janela "Novo token de registry" | `new_registry_token_window.gv` | modal em `screens/docker.js` | 5. cromo de plataforma |
| Janela de logs ao vivo | `log_window.gv` | painel inline (`serviceLogLines`) | 5. cromo de plataforma |

## Lacuna funcional encontrada e corrigida nesta sessão

**Aba "General" do detalhe de serviço, webui**: até este ponto só mostrava
Origem/Porta/Réplicas/Container ao vivo (texto, sem edição) + botão Remover.
A GUI iced tem, na mesma aba (`service.gv` ~L192-566), o editor completo de
origem:

- **Compose**: textarea + Salvar/Cancelar.
- **Provider Git**: URL do repo/imagem, branch, porta, username,
  credentials (nome de secret), build path, watch paths, submodules,
  dockerfile, context path, build stage.
- **Provider Gitea/GitHub**: picker conta conectada → repositório → branch
  (via `GitProviderList`/`GitRepoList`/`GitBranchList`), com os mesmos
  campos de build da sub-aba Git.
- **Provider Zip**: upload de arquivo com Dockerfile na raiz.

Implementado em `webui/screens/service_detail.js` + `webui/index.html` (aba
General) + `webui/net/api.js` (`Api.uploadArchive`, novo — mais simples que
o client iced porque o `<input type=file>` do browser já dá os bytes crus,
sem o round-trip por base64 que o Luau precisa para `fetch("file://…")`) +
`webui/fmt.js` (`looksLikeGitUrl`, porta de `helpers.luau`). Validado
visualmente via Chrome headless (Alpine store seedado à mão) nas 4
variantes — Git, Gitea/GitHub, Zip, Compose. Bateu no gotcha de
`flex-shrink:0` (ver `docs/` — memória `webui_flex_shrink_gotcha`):
containers novos dentro de `.scroll_fill` sem essa propriedade colapsam
altura e o conteúdo seguinte sobrepõe visualmente; corrigido usando a classe
`.form_panel`, que já embute `flex-shrink:0`.

## Lacunas de cobertura de teste (2.B — corrigidas nesta sessão)

`crates/rustploy-gui/tests/templates_render.rs` cobria cada `view`/`tab` no
seu estado *default*, mas várias sub-abas com estado próprio nunca eram
avaliadas:

- `docker_tab`: só o default (`containers`) era exercitado; `images`,
  `volumes`, `networks` e `registry` (nos dois branches — lista de repos e
  repo selecionado com tags) nunca tinham sido avaliados.
- `settings_tab="maintenance"` (limpeza automática de Docker): nunca
  avaliado; as 3 recorrências (`interval`/`daily`/`weekly`) mostram campos
  diferentes.
- `prov_tab` (sub-aba Provider da aba General do serviço): só `"gitea"` era
  exercitado; `"git"` e `"zip"` nunca. Também o branch
  `svc_source_kind="Compose"` (troca o bloco inteiro pelo editor de texto)
  nunca tinha sido avaliado.

Todos adicionados a `all_screens_and_service_tabs_render` nesta sessão.

## Teste headless equivalente do lado webui

Implementado nesta sessão em `crates/daemon/src/api/web_ui.rs`
(`#[cfg(test)] mod headless_tests`, 3 testes) — `cargo test -p rustploy
--bin rustployd headless_tests`. Não reimplementa um servidor de estáticos:
sobe um `hyper` mínimo que só chama `web_ui::serve()`, os MESMOS bytes
(minificados+gzipados por `build.rs`) que o daemon serve em produção, e
dirige um `google-chrome` headless de verdade via `chromiumoxide` (fala CDP
direto — sem precisar de `chromedriver`). Bypassa login/SSE semeando
`Alpine.store('app')` direto (equivalente a `GlacierUI::define_data` do
teste iced) e varre: as 8 telas de nível superior, `project` (com
`selectedProjectId`) e `service` (com `serviceDetail` semeado à mão, já que
esse dado vem de `fetchServiceDetail` — não do snapshot) — incluindo as 3
sub-abas Provider e a origem Compose da nova aba General.

Duas armadilhas encontradas ao escrever este teste, guardadas nos
comentários do código:

- **`user_data_dir` do Chrome precisa ser único por teste** — `cargo test`
  roda os 3 em threads concorrentes; sem isso todos apontam pro mesmo
  perfil default do chromiumoxide e o segundo Chrome a abrir morre com
  "Failed to create SingletonLock".
- **`x-show`/`x-if` do Alpine não atualizam o DOM sincronamente** — a
  mutação do store dispara o recompute, mas a aplicação no DOM é agendada
  num microtask do scheduler do Alpine; um `evaluate()` (round-trip CDP)
  imediatamente após mudar o estado pode chegar antes desse microtask
  rodar. `visible()` faz polling (até 1s) em vez de checar uma vez só.
- **Nome de campo local colidindo entre componentes Alpine diferentes**
  (achado real, não flakiness): tanto `serviceDetail` quanto o wizard
  "Novo serviço" (`newService`) usam `composeText` como nome do campo
  local do textarea de compose colado — um `document.querySelector` sem
  escopo pegava o textarea ERRADO (do wizard, fora de tela). Inofensivo em
  produção (cada `Alpine.data` tem seu próprio escopo), mas exige seletor
  escopado (`[x-data="serviceDetail"] textarea[...]`) no teste.

`chromiumoxide` puxa `reqwest` como dependência transitiva (mesmo sem a
feature `fetcher`) — o projeto evita `reqwest` deliberadamente (ver
CLAUDE.md/memória), mas aqui é só `dev-dependency` de `crates/daemon`
(nunca no binário publicado); usuário aprovou explicitamente essa exceção.
