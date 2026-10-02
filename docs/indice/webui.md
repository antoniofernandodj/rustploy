# Índice: webui (HTML + Alpine.js) servida pelo daemon

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> HTML: `seções` = comentários `── X ──`; `x-data` = componentes Alpine; `chama` = métodos usados em `@click`/`@submit`/….
> JS: `Alpine.store/data("x")` abre um bloco com seus métodos indentados; `get:` lista os getters; `function x(a)` = função de módulo.

## rustploy-daemon/crates/daemon/webui/

### app.css — Rustploy — web UI stylesheet.

### app.js — único <script type="module"> carregado por index.html.
function loadPrefs()
function savePrefs(p)
  Alpine.store("app")
    toast(message, kind, durationMs)
    toastOk(message)
    toastErr(message)
    toastWarn(message)
    toastResult(r, okMessage) — Desfecho de um rpc num passo só: `okMessage` omitido = sucesso silencioso (a tela já mostra o result…
    dismissToast(id)
    serviceNameById(id)
    nav(view)
    persistPrefs()
    connect()
    disconnect()
    openStream()
    onStreamEvent(msg)
    applyBusEvent(ev) — Um evento do bus (mesmo formato que stream.luau::apply_bus trata) — unit variants chegam como string…
    applySnapshot(msg)
    searchChanged(v)
    stopAll()
    clearFinished()
    refreshNow()
    createProject(name, description)
    updateProject(id, name, description)
    deleteProject(id)
    openProject(id)
    saveProjectEnv(envVars, envComments)
    addSecret(name, value)
    deleteSecret(name)
    openNewService()
    fetchWizardCatalog(search) — Catálogos do wizard (bancos/brokers/templates) — buscados uma vez ao abrir a tela "Novo serviço" (ve…
    wizardCreate(req) — `req` é o `WizardCreateReq` completo (ver screens/new_service.js) — o daemon (shared::wizard::build_…
    createServiceDirect(name, source, port, domain) — `source` já é o `ServiceSource` externally-tagged.
    openService(id)
    fetchServiceDetail(id)
    saveServiceSpec(spec, okMsg)
    deleteService(id)
    stopService(id) — Botão "Parar" do card de serviço (grid do projeto) — porta de handlers/projects.luau::svc_stop_id.
    stopAndDeleteService(id) — Botão "Remover" do card de serviço (grid do projeto) — porta de handlers/projects.luau::stop_delete_…
    deployStart()
    deployAbort(deploymentId)
    deployRollback()
    deleteDeployment(deploymentId)
    queueCancel(deploymentId) — Cancela um deploy que ainda espera na fila — reaproveita DeployAbort (o daemon já trata remoção da f…
    queuePromote(deploymentId)
    queueTogglePause()
    toastPrune(r) — Toast de um prune, com o Response::PruneResult{count,reclaimed_bytes} quando o daemon o devolve; sem…
    dockerPruneContainers()
    dockerPruneImages()
    dockerPruneVolumes()
    dockerPruneNetworks()
    dockerRemoveContainer(id)
    dockerRemoveImage(id)
    dockerRemoveVolume(name)
    dockerRemoveNetwork(id)
    dockerSetTab(tab) — Troca a sub-aba Docker; ao entrar em "registry" busca os tokens (não vêm no snapshot periódico, dife…
    registryOpenRepo(name)
    registryCloseRepo()
    registryRefreshTokens() — Sem rpcChecked de propósito (mesmo comportamento silencioso do Luau — falha aqui não é acionável pel…
    registryRmTag(tag)
    registryRmRepo(name)
    registryGc()
    registryRmToken(name)
    registryCreateToken(name, scope) — Devolve {ok, secret} pro modal de "novo token" — o segredo só existe nesta resposta, nunca mais é re…
    jobRunNow(id)
    jobRunCancel(jobRunId) — Cancela um job_run em execução (ver docs/plano-cancelamento-de-jobs.md): mata o processo `docker com…
    jobToggle(id) — Reenvia o Job inteiro (só `enabled` inverte) — o daemon não tem um PATCH parcial; mesma limitação do…
    jobDelete(id)
    jobCreate(payload) — `payload` = { project_id, trigger_service_id, name, compose, main_service, recurrence }.
    openNewJob()
    closeNewJob()
    njobPickProject(id, name)
    njobPickService(id, name)
    njobPickNoService()
    njobBack()
    njobSetSourceTab(kind) — ── Fonte do compose: aba "Compose" x aba "Git" (picker conta→repo→branch) ── Porta de njob_source/nj…
    njobGitProviderPick(id)
    njobGitRepoPick(fullName)
    buildNjobRecurrence() — Monta `recurrence` (Option<Recurrence>, externally-tagged) a partir de njobKind.
    njobCreate()
    openEditJob(id) — Abre o mesmo modal do wizard, mas em modo edição: pula pro passo "form" já preenchido com o job exis…
    openJobLogs(jobRunId)
    closeJobLogs()
    startJobLogs(jobRunId) — Logs ao vivo de UMA execução de job — mesmo par seed+SSE de startServiceLogs/stopServiceLogs, aponta…
    stopJobLogs()
    loadSettings()
    settingsSave()
    gpRefresh()
    gpConnect() — Mesmas validações de handlers/settings.luau::gp_connect: GitHub cai pro github.com se a Base URL vie…
    gpDelete(id)
    iacExport()
    iacImport() — 3 formas de resposta possíveis (mesma distinção de handlers/settings.luau::iac_import): MissingEnvVa…
    dcApplyConfig(cfg) — Recorrência (Option<Recurrence>, externally-tagged) → campos do formulário — mesmo formato do unpack…
    dcLoad()
    dcBuildRecurrence()
    dcSave()
    dcRunNow() — Botão "Executar agora": roda os recursos marcados fora do horário agendado, independente do interrup…
    serviceStop()
    serviceReload()
    cleanLogEntry(e) — LogEntry/LogLine cru → { stream, line, timestamp } com ANSI limpo e truncado (defesa contra uma linh…
    startServiceLogs()
    stopServiceLogs()
    setServiceTab(tab)
    get: gpRedirect, njobProjects, njobServicesFiltered

### directives.js — diretivas Alpine próprias da webui.
function registerDirectives(Alpine) — directives.js — diretivas Alpine próprias da webui.

### fmt.js — timestamps, durações e paleta de estado.
function toEpochMs(iso) — epoch (ms) de um RFC3339 ("...Z" ou offset).
function timeHms(iso) — "HH:MM:SS" local.
function dateDmHms(iso) — "dd/mm HH:MM:SS" local.
function dateDmHm(iso) — "dd/mm HH:MM" local (sem segundos — usado em listas Docker/Registry).
function matchesTerm(term, fields) — `term` já em minúsculas; casa se algum campo (string) contém `term` (substring, case-insensitive).
function hmJoin(hour, minute) — (hour, minute) -> "HH:MM", saturando na faixa válida.
function hmSplit(hm) — "HH:MM" -> [hour, minute].
function fmtSecs(secs) — Ns ou Mm Ns.
function fmtUptime(secs) — dd hh mm / hh mm / mm ss, o maior campo não-zero primeiro.
function fmtDuration(dep) — Duração de um deployment (finished_at ou agora) − started_at.
function stateLabelKind(state) — DeployState → (rótulo, kind semântico p/ .state_<kind>).
function serviceStatusLabelKind(status) — ServiceStatus (string ou `{Error: "..."}`) → (rótulo, kind).
function shortReason(msg, max) — Tamanho de bytes legível ("—" para 0/ausente).
function fmtBytes(b)
function sourceSummary(source) — Resumo curto da origem de um serviço (ServiceSource externally-tagged).
function looksLikeGitUrl(u) — Heurística "isso parece uma URL de Git, não uma imagem de registry".
function parseSecretRef(v) — `<secret:NOME>` ou `secret:NOME` → "NOME" (ou null se não for referência).
function dotenvFromVars(vars, comments) — env_vars + env_comments → texto `.env` (KEY=VALUE, secrets como `<secret:NOME>`, comentários `# ...`…
function envRowsWithComments(vars, comments) — env_vars + env_comments → linhas pra exibição na lista normal (fora do editor `.env` bruto), comentá…
function stripAnsi(s) — Remove sequências de escape ANSI (cor/cursor/erase) de uma linha de log.
function parseDotenv(text) — Texto `.env` → { vars, comments } (env_vars/env_comments do ServiceSpec/Project).
function safeName(name) — Normaliza um nome de serviço para `[a-z0-9_]`, mesmo algoritmo de `crate::normalize_name` (Rust) / `…
function internalScheme(dbKind)
function composeHost(content, ingressService) — Chave do serviço que recebe o tráfego dentro de um compose: `ingress_service` se declarado, senão a …
function internalUrl(dbKind, safe, port, composeHostName) — URL de conexão dentro da rede Docker do daemon (`rp_<safe>:<porta>`, com esquema por tipo de banco).
function envPlain(vars, key)
function dbCredentials(dbKind, vars) — (database, user, password) lidos das env vars conhecidas do banco.
function withDbCredentials(base, database, user, password)
function pct(s)
function userinfo(user, password)
function externalScheme(k)
function dbConnectionUrl(dbKind, host, port, database, user, password)
function urlHost(apiUrl)
function externalUrl(domain, tls, hostPort, dbKind, apiUrl, envVars) — URL de conexão externa: domínio HTTP tem prioridade; sem domínio, cai pro passthrough TCP (host_port…
function pairList(services) — `msg.services` (`[{project_name, service}]`) → `[{svc, proj}]`.
function domainRoutes(spec) — `spec.domains` se houver, senão o legado `domain`/`tls_enabled`.
function ingressRows(pairs) — Ingress: uma linha por rota de domínio (não filtrado pela busca).
function hostPortRows(pairs) — Portas TCP de host: uma linha por serviço com `host_port` configurado.
function monitoringRows(pairs, metricsById) — Monitoring: uma linha por serviço COM métricas vivas (`metricsById[id]` só existe depois do primeiro…
function progressBar(percent, width) — Barra de progresso em blocos (`█`/`░`).
function engActiveRows(active) — Deploy Engine: "Executando agora".
function engQueuedRows(queued) — Deploy Engine: "Na fila" (o primeiro é o próximo a rodar).
function dockerContainerRows(list, term) — Containers do host (rodando + parados).
function containerStateLabel(state) — Rótulo/kind de um estado bruto do Docker ("running","exited",...).
function containerStopped(state) — Porta literal de fmt/util.luau::container_stopped — só os parados podem ser removidos (o Docker recu…
function dockerImageRows(list, term)
function dockerVolumeRows(list, term)
function dockerNetworkRows(list, term)
function registryRepoRows(list, term) — Lista de repositórios (filtrada pela busca global).
function registryTagRows(list) — Tags de UM repositório (sem filtro — lista pequena, buscada sob demanda).
function registryTokenRows(list) — Tokens de acesso Basic auth (sem filtro — lista pequena).
function recurrenceLabel(r) — Resumo textual de uma `Recurrence?` (nil = só manual).
function jobRunStateLabel(run) — Rótulo/kind da última execução de um job.
function jobSummaryRows(list, term, inflight) — Tela global "Schedules": uma linha por job, de todos os projetos.
function oauthRedirectUri(base, kind) — Redirect URI do OAuth (`{base}/oauth/{gitea|github}/callback`).
function gitKindLabel(kind)
function gitProviderRows(list) — Porta literal de fmt/git.luau::git_providers.
function dockerCleanupResourceLabel(name)
function dockerCleanupLastRunSummary(lr) — `lr`: `DockerCleanupLastRun?` (`{ at, results: [{ resource, count, reclaimed_bytes, error }] }`).
function engRecentRows(recent) — Deploy Engine: "Histórico 24h".

### index.html — Casca única da webui (Alpine.js): login, shell e todas as telas, uma seção por view, cada uma com o …
seções: Login, Shell (sidebar + topbar + conteúdo), Deploy Engine, Monitoring, Ingress, Docker, Schedules, Modais globais de Jobs, Settings, Projects, Projeto aberto, Novo serviço (wizard por passos, porta de new_service.gv), Detalhe de serviço
x-data: dashboard, deployEngine, docker, ingress, login, monitoring, newService, projectDetail, projects, schedules, serviceDetail, settings
chama: String, abortDeployment, addDomain, addEnvVar, cancel, cancelCompose, cancelEdit, clearFinished, closeBuildLog, closeEnvText, closeJobLogs, closeNewJob, closeTokenModal, copyToClipboard, createMdb, dcRunNow, dcSave, delDomain, delEnvVar, deleteMdb, deleteProject, deleteSecret, deleteService, deployRollback, deployStart, discardOldDb, disconnect, dismissToast, dockerPruneContainers, dockerPruneImages, dockerPruneNetworks, dockerPruneVolumes, dockerRemoveContainer, dockerRemoveImage, dockerRemoveNetwork, dockerRemoveVolume, dockerSetTab, giteaProviderPick, giteaRepoPick, gotoApp, gotoBroker, gotoCompose, gotoDb, gotoTemplate, gotoType, gpConnect, gpDelete, gpRefresh, iacExport, iacImport, initAdvForm, initGeneralForm, initHcForm, jobDelete, jobRunCancel, jobRunNow, jobToggle, loadMigration, nav, njobBack, njobCreate, njobGitProviderPick, njobGitRepoPick, njobPickNoService, njobPickProject, njobPickService, njobSetSourceTab, ntokCreate, onArchiveFileChange, open, openEditJob, openEnvText, openJobLogs, openNewJob, openNewService, openProject, openService, openTokenModal, pdcAdd, pdcDel, pdcMove, persistPrefs, pickBroker, pickDb, pickTemplate, queueCancel, queuePromote, queueTogglePause, registryCloseRepo, registryGc, registryOpenRepo, registryRmRepo, registryRmTag, registryRmToken, removeDeployment, renameService, rollbackMigration, saveAdvanced, saveCompose, saveEdit, saveEnvText, saveHealthcheck, saveSource, searchChanged, serviceReload, serviceStop, setProvTab, setServiceTab, setShared, settingsSave, showMdb, startEdit, startMigration, stopAll, stopAndDeleteService, stopService, submit, submitApp, submitBroker, submitCompose, submitDb, submitNew, submitSecret, submitTemplate, uploadArchive, viewBuildLog, writeText

## rustploy-daemon/crates/daemon/webui/net/

### api.js — cliente HTTP/JSON do daemon.
class Api
  constructor(baseUrl, token)
  headers()
  rpc(cmd) — Executa um Command.
  rpcChecked(cmd) — Como rpc(), mas trata `Response::Err { code, message }` como falha.
  uploadArchive(serviceId, file) — `POST /api/services/<id>/archive` — corpo binário cru (não é RPC JSON).

### sse.js — consumidor de endpoints SSE do daemon: o firehose `/api/events` (porta de crates/rustploy-gui/views/…
function openStream(baseUrl, token, path, handlers) — Abre a stream em `path` (relativo a `baseUrl`, ex.

## rustploy-daemon/crates/daemon/webui/screens/

### dashboard.js — tela "Deployments" (view padrão do shell).
  Alpine.data("dashboard")
    get: store, rows

### deploy_engine.js — tela "Deploy Engine": fila global (um deploy por vez), execução em andamento e histórico das últimas…
  Alpine.data("deployEngine")
    get: store, engine, active, queued, recent, paused, uptime, successCount, failedCount, totalCount

### docker.js — tela "Docker": containers/imagens/volumes/networks do host inteiro (não só recursos geridos pelo rus…
  Alpine.data("docker")
    openTokenModal()
    closeTokenModal()
    ntokCreate()
    get: store, containers, images, volumes, networks, repos, tags, tokens, registryHost, registryStatusLabel

### ingress.js — tela "Ingress": rotas ativas no reverse proxy (por domínio) e portas TCP de host expostas diretament…
  Alpine.data("ingress")
    get: store, pairs, routes, hostPorts

### login.js — tela de login.
  Alpine.data("login")
    submit()
    get: store

### monitoring.js — tela "Monitoring": uso de CPU/memória do host e por container.
  Alpine.data("monitoring")
    get: store, rows

### new_service.js — wizard "Novo serviço", porta de new_service.gv (client iced): passo pick_type → passo específico por…
  Alpine.data("newService")
    ensureCatalog()
    gotoType() — ── Navegação por passos ─────────────────────────────────────────
    gotoApp()
    gotoCompose()
    gotoDb()
    pickDb(db)
    gotoBroker()
    pickBroker(b)
    gotoTemplate()
    pickTemplate(t)
    cancel()
    baseReq(kind, id) — ── Submissões ────────────────────────────────────────────────────
    submitApp()
    submitCompose()
    submitDb()
    submitBroker()
    submitTemplate()
    get: store, filteredTemplates

### project_detail.js — projeto aberto (view=project_services no client iced): sub-abas Serviços/Variáveis/Secrets/Jobs.
function primaryContainer(svc) — Container "primário" de um serviço pra exibir no card: o live, senão o primeiro em execução, senão o…
  Alpine.data("projectDetail")
    startEdit()
    cancelEdit()
    saveEdit()
    addEnvVar()
    delEnvVar(key)
    openEnvText()
    closeEnvText()
    saveEnvText()
    submitSecret()
    get: store, project, services, canDelete, jobs, envVars, secrets

### projects.js — tela "Projects": grid de cards + criar/editar/remover.
  Alpine.data("projects")
    submitNew()
    startEdit(row)
    cancelEdit()
    saveEdit()
    get: store, rows

### schedules.js — tela "Schedules": jobs one-shot (docker-compose) agendados ou manuais, de todos os projetos.
  Alpine.data("schedules")
    get: store, rows

### service_detail.js — detalhe de um serviço.
  Alpine.data("serviceDetail")
    init() — `x-show` mantém este componente montado por toda a sessão — abrir um serviço não recria o Alpine.dat…
    loadMigration()
    startMigration()
    rollbackMigration()
    discardOldDb()
    loadSharedState()
    setShared(on)
    showMdb(d)
    createMdb()
    deleteMdb(d)
    loadConnUrl()
    initGeneralForm()
    renameService() — Unicidade dentro do projeto é checada no daemon (a mensagem volta no toast).
    saveCompose()
    cancelCompose()
    setProvTab(tab)
    giteaProviderPick(id)
    giteaRepoPick(fullName)
    saveSource() — Porta de handlers/services.luau::gen_save — mesma heurística looksLikeGitUrl decide Git vs Registry …
    onArchiveFileChange(event)
    uploadArchive()
    copyToClipboard(text)
    addEnvVar()
    delEnvVar(key)
    openEnvText()
    closeEnvText()
    saveEnvText()
    addDomain()
    delDomain(domain)
    initHcForm() — Popula o form local a partir do spec atual — chamado ao abrir a aba (o form é editável, não reativo …
    saveHealthcheck()
    initAdvForm()
    saveAdvanced()
    pdcAdd()
    pdcDel(jobId)
    pdcMove(jobId, delta) — Sem drag-and-drop na web UI (sem lib de DnD): reordena com botões mover-pra-cima/baixo — mesmo resul…
    viewBuildLog(deploymentId)
    closeBuildLog()
    abortDeployment(deploymentId)
    removeDeployment(deploymentId)
    get: store, canMigrate, migSteps, canShare, isShared, svc, statusLabel, statusKind, sourceText, isCompose, renameNote, liveContainers, connectionInfo, envVars, domains, runArgsText, preDeployQueueIds, preDeployChecks, preDeployAvailableJobs, deployments

### settings.js — tela "Settings": Web Server / Git / Infra as Code.
  Alpine.data("settings")
    get: store, gitProviderRows

## rustploy-daemon/crates/daemon/webui/

### sw.js — service worker do PWA Rustploy.
