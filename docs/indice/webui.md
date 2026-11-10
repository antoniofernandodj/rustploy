# Índice: webui (HTML + Alpine.js) servida pelo daemon

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> HTML: `seções` = comentários `── X ──`; `x-data` = componentes Alpine; `chama` = métodos usados em `@click`/`@submit`/….
> JS: `Alpine.store/data("x")` abre um bloco com seus métodos indentados; `get:` lista os getters; `function x(a)` = função de módulo.

## rustploy-daemon/crates/daemon/webui/

### app.css — Rustploy — web UI stylesheet.

### app.js — único <script type="module"> carregado por index.html.
function loadPrefs()
function savePrefs(preferences)
  Alpine.store("app")
    toast(message, kind, durationMs)
    copyText(text, what) — Copia `text` e avisa por toast; falha (sem permissão/contexto inseguro) vira toast de erro.
    toastOk(message)
    toastError(message)
    toastWarn(message)
    toastResult(result, okMessage) — Desfecho de um rpc num passo só: `okMessage` omitido = sucesso silencioso (a tela já mostra o result…
    dismissToast(toastId)
    serviceNameById(serviceId)
    nav(view)
    persistPrefs()
    connect()
    disconnect()
    openStream()
    onStreamEvent(streamMessage)
    applyBusEvent(busEvent) — Um evento do bus (mesmo formato que stream.luau::apply_bus trata) — unit variants chegam como string…
    applySnapshot(snapshot)
    searchChanged(searchText)
    stopAll()
    clearFinished()
    refreshNow()
    createProject(name, description)
    updateProject(projectId, name, description)
    deleteProject(projectId)
    openProject(projectId)
    saveProjectEnv(envVars, envComments)
    addSecret(name, value)
    deleteSecret(name)
    openNewService(step) — `step` opcional: "import" abre direto no passo Importar (ícone do card).
    closeNewService()
    openExportWin(serviceId)
    closeExportWin()
    openProjectWin(project)
    closeProjectWin()
    fetchWizardCatalog(search) — Catálogos do wizard (bancos/brokers/templates) — buscados uma vez ao abrir a tela "Novo serviço" (ve…
    wizardCreate(createRequest) — `req` é o `WizardCreateReq` completo (ver screens/new_service.js) — o daemon (shared::wizard::build_…
    createServiceDirect(name, source, port, domain) — `source` já é o `ServiceSource` externally-tagged.
    openService(serviceId)
    fetchServiceDetail(serviceId)
    saveServiceSpec(spec, okMessage)
    deleteService(serviceId)
    stopService(serviceId) — Botão "Parar" do card de serviço (grid do projeto) — porta de handlers/projects.luau::svc_stop_id.
    stopAndDeleteService(serviceId) — Botão "Remover" do card de serviço (grid do projeto) — porta de handlers/projects.luau::stop_delete_…
    deployServiceFromList(serviceId) — Atalho "Deploy" do painel "Em execução": dispara o deploy de um serviço sem abrir a tela dele.
    requestServiceDeploy(serviceId, fromDetail) — Pede o plano (ordem das dependências).
    requestProjectDeploy() — Botão "Deploy" do projeto: sempre mostra a ordem de tudo.
    openServicesReorder() — Abre o modal com os serviços do projeto na ordem atual do daemon.
    closeServicesReorder()
    servicesReorderMove(fromId, toId) — Move `fromId` para a posição de `toId` (solta A sobre B: A passa a ficar antes de B).
    servicesReorderStep(id, delta) — Botões ↑/↓: troca com o vizinho (alternativa ao arrastar).
    saveServicesReorder() — "Reordenar": grava no daemon (vai para o project.yml) e os cards se reordenam.
    closeDeployPlan()
    confirmDeployPlan() — "Confirmar"/"Aceitar": enfileira o lote na ordem do plano.
    deployPlanOnlyService() — "Só o serviço": ignora as dependências.
    deployPlanLine(item, index) — Texto de uma linha do plano: "1.
    deploySingle(serviceId, fromDetail)
    deployStart()
    deployStartNow()
    deployAbort(deploymentId)
    deployRollback()
    deleteDeployment(deploymentId)
    queueCancel(deploymentId) — Cancela um deploy que ainda espera na fila — reaproveita DeployAbort (o daemon já trata remoção da f…
    queuePromote(deploymentId)
    queueReorder(order)
    queueTogglePause()
    toastPrune(result) — Toast de um prune, com o Response::PruneResult{count,reclaimed_bytes} quando o daemon o devolve; sem…
    dockerPruneContainers()
    dockerPruneImages()
    dockerPruneVolumes()
    dockerPruneNetworks()
    dockerStopContainer(containerId) — Para um container órfão ("removido no git").
    dockerRemoveContainer(containerId)
    dockerRemoveImage(imageId)
    dockerRemoveVolume(name)
    dockerRemoveNetwork(networkId)
    dockerSetTab(tab) — Troca a sub-aba Docker; ao entrar em "registry" busca os tokens (não vêm no snapshot periódico, dife…
    registryOpenRepo(name)
    registryCloseRepo()
    registryRefreshTokens() — Sem rpcChecked de propósito (mesmo comportamento silencioso do Luau — falha aqui não é acionável pel…
    registryRemoveTag(tag)
    registryRemoveRepo(name)
    registryGc()
    registryRetentionSet(config)
    registryRemoveToken(name)
    registryCreateToken(name, scope) — Devolve {ok, secret} pro modal de "novo token" — o segredo só existe nesta resposta, nunca mais é re…
    jobRunNow(jobId)
    jobRunCancel(jobRunId) — Cancela um job_run em execução (ver docs/plano-cancelamento-de-jobs.md): mata o processo `docker com…
    jobToggle(jobId) — Reenvia o Job inteiro (só `enabled` inverte) — o daemon não tem um PATCH parcial; mesma limitação do…
    jobDelete(jobId)
    jobWarm(jobId) — Aquece o job (baixa as imagens que faltam e popula o cache de build) sem executá-lo.
    jobRebuild(jobId) — Reconstrói o job do zero (`JobWarm` com `force`): rebaixa as imagens e refaz o build sem cache, mesm…
    serviceWarm(serviceId) — Aquece o serviço (constrói a imagem para popular o cache de build) sem fazer deploy: o container no …
    openJobActions(jobId)
    closeJobActions()
    jobCreate(payload) — `payload` = { project_id, trigger_service_id, name, compose, main_service, recurrence }.
    openNewJob()
    closeNewJob()
    newJobPickProject(projectId, name)
    newJobPickService(serviceId, name)
    newJobPickNoService()
    newJobBack()
    newJobSetSourceTab(kind) — ── Fonte do compose: aba "Compose" x aba "Git" (picker conta→repo→branch) ── Porta de njob_source/nj…
    newJobGitProviderPick(providerId)
    newJobGitRepoPick(fullName)
    buildNewJobRecurrence() — Monta `recurrence` (Option<Recurrence>, externally-tagged) a partir de newJobKind.
    newJobCreate()
    openEditJob(jobId) — Abre o mesmo modal do wizard, mas em modo edição: pula pro passo "form" já preenchido com o job exis…
    openJobLogs(jobRunId)
    closeJobLogs()
    startJobLogs(jobRunId) — Logs ao vivo de UMA execução de job — mesmo par seed+SSE de startServiceLogs/stopServiceLogs, aponta…
    stopJobLogs()
    loadSettings()
    updateCheck(force) — Pergunta ao daemon se há release mais nova (Command::CheckUpdate: só leitura, com cache de horas no …
    applyUpdateInfo(updateInfo) — Preenche a aba Atualizações e o selo com um `UpdateInfo`.
    updateProgressText(updateProgress) — Texto do que o daemon sabe sobre a atualização em curso ou a última tentativa (Command::UpdateStatus…
    updateLoadProgress() — Busca o estado da atualização; devolve o `stage` (ou `null` se o daemon não respondeu, por exemplo e…
    updateApply() — Atualiza o daemon pelo app (Command::ApplyUpdate).
    updateFollowProgress() — Acompanha a atualização até um estado final.
    updateRollback() — Volta os binários para os do backup da última atualização (Command::UpdateRollback).
    updateCheckNow()
    stateGitTimeLabel(isoTimestamp) — "dd/mm HH:MM:SS" local de um instante RFC 3339 do status, ou "—".
    stateGitPendingDeploy(serviceId) — O serviço mudou por um commit externo e ainda não foi redeployado.
    stateGitOpen() — Abre a aba da sincronização e carrega o formulário a partir do status atual.
    stateGitFormLoad()
    stateGitAuditRefresh() — A auditoria vem no snapshot: atualizar é reler o snapshot.
    stateGitConfigure()
    stateGitSyncNow()
    stateGitResume() — Depois de resolver à mão por SSH: o daemon relê a pasta e valida antes de voltar a andar.
    stateGitResolve(choice) — Resolve a divergência descartando um dos lados (com cópia de segurança do descartado).
    stateGitDismissRemoval(removalId) — Esquece a marca "removido no git" (a pessoa decidiu manter o que ainda roda, ou já limpou).
    stateGitAllowFinding(finding, allowed) — Marca/desmarca uma suspeita da auditoria como "não é segredo" (certezas não podem).
    settingsSave()
    gitProviderRefresh()
    gitProviderConnect() — Mesmas validações de handlers/settings.luau::gp_connect: GitHub cai pro github.com se a Base URL vie…
    gitProviderDelete(providerId)
    infraDownload() — "Baixar infra": o daemon devolve os mesmos arquivos que o `state_git` grava no repositório (secrets …
    dockerCleanupApplyConfig(config) — Recorrência (Option<Recurrence>, externally-tagged) → campos do formulário — mesmo formato do unpack…
    dockerCleanupLoad()
    dockerCleanupBuildRecurrence()
    dockerCleanupSave()
    dockerCleanupRunNow() — Botão "Executar agora": roda os recursos marcados fora do horário agendado, independente do interrup…
    serviceStop()
    serviceReload()
    cleanLogEntry(logEntry) — LogEntry/LogLine cru → { stream, line, timestamp } com ANSI limpo e truncado (defesa contra uma linh…
    historyToLines(entries, previousContainer) — Entradas do LogsHistory (ordem cronológica) → linhas da tela, com um separador onde o container muda…
    firstLogContainer() — Container da primeira linha real da tela (para o separador ao prepor histórico).
    serviceLogLoadOlder() — "Carregar anteriores": a página de log guardado anterior ao que a tela já mostra.
    serviceLogExecutionsLoad() — Lê as execuções do serviço: a atual (ou, parado, a mais recente) alimenta o painel ao vivo, as outra…
    serviceLogToggle(key)
    serviceLogToggleAll()
    serviceLogSelectedBytes()
    serviceLogExecutionOpen(row) — Abre a janela de uma execução passada: só as linhas dela, com data e hora, sem ao vivo.
    serviceLogExecutionLoadOlder() — "Carregar anteriores" dentro da janela de uma execução passada.
    serviceLogExecutionsDelete(keys, message) — Exclui execuções (irreversível) e recarrega a lista.
    serviceLogExecutionDelete(row)
    serviceLogSelectedDelete()
    startServiceLogs()
    stopServiceLogs()
    setServiceTab(tab)
    get: gitProviderRedirect, servicesReorderRows, jobActionsRow, newJobProjects, newJobServicesFiltered, stateGitStatus, stateGitAuditFindings, stateGitAlertMessage

### busy.js — feedback imediato e trava de clique para toda ação assíncrona.
const isBusy(element)
function arm(button)
function lock(button)
function wrap(originalFunction)
function wrapToast(originalFunction) — Toast de resposta: libera os botões ocupados (fora de eventos do stream).
function wrapStream(originalFunction) — Evento SSE: os toasts de dentro não são resposta a um clique.
function track(obj)
function registerBusy(Alpine)

### directives.js — diretivas Alpine próprias da webui.
function registerDirectives(Alpine) — directives.js — diretivas Alpine próprias da webui.

### format.js
function toEpochMs(isoTimestamp) — epoch (ms) de um RFC3339 ("...Z" ou offset).
function timeHms(isoTimestamp) — "HH:MM:SS" local.
function dateDayMonthHourMinuteSecond(isoTimestamp) — "dd/mm HH:MM:SS" local.
function dateTimeFull(isoTimestamp) — "dd/mm/aaaa HH:MM:SS" local (log de uma execução antiga: a data importa).
function logExecutionRows(executions) — Linhas da lista "Execuções anteriores" (LogsExecutions): rótulos prontos para a tela.
function dateDayMonthHourMinute(isoTimestamp) — "dd/mm HH:MM" local (sem segundos — usado em listas Docker/Registry).
function matchesTerm(term, fields) — `term` já em minúsculas; casa se algum campo (string) contém `term` (substring, case-insensitive).
function hourMinuteJoin(hour, minute) — (hour, minute) -> "HH:MM", saturando na faixa válida.
function hourMinuteSplit(hourMinuteText) — "HH:MM" -> [hour, minute].
function formatSeconds(secs) — Ns ou Mm Ns.
function formatUptime(secs) — dd hh mm / hh mm / mm ss, o maior campo não-zero primeiro.
function formatDuration(deployment) — Duração de um deployment (finished_at ou agora) − started_at.
function stateLabelKind(state) — DeployState → (rótulo, kind semântico p/ .state_<kind>).
function serviceStatusLabelKind(status) — ServiceStatus (string ou `{Error: "..."}`) → (rótulo, kind).
function shortReason(message, max) — Tamanho de bytes legível ("—" para 0/ausente).
function formatBytes(bytes)
function sourceSummary(source) — Resumo curto da origem de um serviço (ServiceSource externally-tagged).
function looksLikeGitUrl(url) — Heurística "isso parece uma URL de Git, não uma imagem de registry".
function parseSecretRef(text) — `<secret:NOME>` ou `secret:NOME` → "NOME" (ou null se não for referência).
function envValueFromDraft(draft, wasSecret) — O valor de uma env como o `EnvValueSet` espera, a partir do texto digitado na edição no lugar: `<sec…
function dotenvFromVars(vars, comments) — env_vars + env_comments → texto `.env` (KEY=VALUE, secrets como `<secret:NOME>`, comentários `# ...`…
function envRowsWithComments(vars, comments) — env_vars + env_comments → linhas pra exibição na lista normal (fora do editor `.env` bruto), comentá…
function stripAnsi(text) — Remove sequências de escape ANSI (cor/cursor/erase) de uma linha de log.
function validEnvKey(key) — Nome aceito: começa com letra ou `_`; o resto são letras, dígitos, `_`, `.` e `-`.
function dotenvErrorsMessage(errors) — Texto de um toast com as 3 primeiras de `errors` e "+N".
function confirmEnvKey(store, key, existingVars) — Valida a chave nova do campo "adicionar": nome inválido avisa e aborta; chave que já existe pede con…
function parseDotenv(text) — Parseia um blob .env em `{vars, comments, errors}`.
function safeName(name) — Normaliza um nome de serviço para `[a-z0-9_]`, mesmo algoritmo de `crate::normalize_name` (Rust) / `…
function internalScheme(databaseKind)
function composeHost(content, ingressService) — Chave do serviço que recebe o tráfego dentro de um compose: `ingress_service` se declarado, senão a …
function internalUrl(databaseKind, safe, port, composeHostName) — URL de conexão dentro da rede Docker do daemon (`rp_<safe>:<porta>`, com esquema por tipo de banco).
function envPlain(vars, key)
function databaseCredentials(databaseKind, vars) — (database, user, password) lidos das env vars conhecidas do banco.
function withDatabaseCredentials(base, database, user, password)
function percentEncode(text)
function userinfo(user, password)
function externalScheme(databaseKind)
function databaseConnectionUrl(databaseKind, host, port, database, user, password)
function urlHost(apiUrl)
function externalUrl(domain, tls, hostPort, databaseKind, apiUrl, envVars) — URL de conexão externa: domínio HTTP tem prioridade; sem domínio, cai pro passthrough TCP (host_port…
function servicePairList(services) — `msg.services` (`[{project_name, service}]`) → `[{svc, proj}]`.
function domainRoutes(spec) — `spec.domains` se houver, senão o legado `domain`/`tls_enabled`.
function ingressRows(pairs) — Ingress: uma linha por rota de domínio (não filtrado pela busca).
function hostPortRows(pairs) — Portas TCP de host: uma linha por serviço com `host_port` configurado.
function monitoringRows(pairs, metricsById) — Monitoring: uma linha por serviço COM métricas vivas (`metricsById[id]` só existe depois do primeiro…
function deployStepIndex(state)
function deployStepper(info) — Linha de passos: [{label, status}] com status done|current|failed|pending.
function deployDetailRows(info) — Detalhe do deploy para o modal: uma linha por transição (estado em que entrou, quanto durou, mensage…
function itemLabel(info) — Deploy Engine: "Executando agora".
function deployEngineActiveRows(active)
function deployEngineQueuedRows(queued) — Deploy Engine: "Na fila" (o primeiro é o próximo a rodar).
function dockerContainerRows(list, term) — Containers do host (rodando + parados).
function containerStateLabel(state) — Rótulo/kind de um estado bruto do Docker ("running","exited",...).
function containerStopped(state) — Porta literal de format/util.luau::container_stopped — só os parados podem ser removidos (o Docker r…
function dockerImageRows(list, term)
function dockerVolumeRows(list, term)
function dockerNetworkRows(list, term)
function registryRepoRows(list, term) — Lista de repositórios (filtrada pela busca global).
function registryTagRows(list) — Tags de UM repositório (sem filtro — lista pequena, buscada sob demanda).
function registryTokenRows(list) — Tokens de acesso Basic auth (sem filtro — lista pequena).
function recurrenceLabel(recurrence) — Resumo textual de uma `Recurrence?` (nil = só manual).
function jobRunStateLabel(run) — Rótulo/kind da última execução de um job.
function retentionHours(text) — Horas de retenção da limpeza do Docker, do formulário → inteiro >= 0. Vazio ou inválido volta ao pad…
function jobWarmLabel(warm) — Rótulo/cor do aquecimento de um job.
function serviceWarmLabel(warm) — Rótulo/cor do aquecimento de um serviço com build.
function jobSummaryRows(list, term, inflight) — Tela global "Schedules": uma linha por job, de todos os projetos.
function oauthRedirectUri(base, kind) — Redirect URI do OAuth (`{base}/oauth/{gitea|github}/callback`).
function gitKindLabel(kind)
function gitProviderRows(list) — Porta literal de format/git.luau::git_providers.
function dockerCleanupResourceLabel(name)
function dockerCleanupLastRunSummary(lastRun) — `lr`: `DockerCleanupLastRun?` (`{ at, results: [{ resource, count, reclaimed_bytes, error }] }`).
function deployEngineRecentRows(recent) — Deploy Engine: "Histórico 24h".

### i18n.js — idioma da webui: inglês por padrão, português (pt-BR) a um clique, escolha lembrada.
function storedLanguage()
function currentLanguage()
function lookup(source)
function fill(text, values)
function t(first, ...values) — `t("texto")` ou o template etiquetado `` t`texto ${valor}` ``.
function normalize(text)
function registerI18n(Alpine)
  Alpine.store("i18n")
    set(code)

### icons.js — conjunto de ícones SVG inline (traço 1.75, estilo Lucide) e a diretiva `x-icon="'nome'"`.
function iconSvg(name, size)
function registerIcons(Alpine)

### index.html — Casca única da webui (Alpine.js): login, shell e todas as telas, uma seção por view, cada uma com o …
seções: Login, Shell (sidebar + topbar + conteúdo), Deploy Engine, Monitoring, Ingress, Docker, Schedules, Modais globais de Jobs, Settings, Projects, Projeto aberto, Novo serviço (wizard por passos, porta de new_service.gv), Janela: novo/editar projeto (wm.js), Janela: exportar serviço (screens/service_bundle.js), Janela: logs ao vivo do serviço (destacável da aba Logs), Janela: log de uma execução passada (histórico, sem ao vivo), Detalhe de serviço
x-data: dashboard, deployEngine, docker, ingress, login, monitoring, newService, projectDetail, projectWin, projects, running, schedules, serviceDetail, serviceExport, serviceImport, settings
chama: String, abortDeployment, addDomain, addEnvVar, addServiceSecret, analyze, cancel, cancelCompose, cancelEnvEdit, chooseEnv, clear, clearFinished, close, closeBuildLog, closeDeployPlan, closeDetail, closeEnvText, closeJobActions, closeJobLogs, closeNewJob, closeNewService, closeProjectWin, closeServicesReorder, closeTokenModal, commitEnvEdit, confirmDeployPlan, confirmImportData, copy, copySecret, copyServiceSecret, copyText, copyToClipboard, create, createSharedDatabase, deleteDomain, deleteEnvVar, deleteProject, deleteSecret, deleteService, deleteServiceSecret, deleteSharedDatabase, dependencyAdd, dependencyDelete, deployPlanOnlyService, deployRollback, deployServiceFromList, deployStart, discardOldDatabase, disconnect, dismissToast, dockerCleanupRunNow, dockerCleanupSave, dockerPruneContainers, dockerPruneImages, dockerPruneNetworks, dockerPruneVolumes, dockerRemoveContainer, dockerRemoveImage, dockerRemoveNetwork, dockerRemoveVolume, dockerSetTab, dockerStopContainer, download, dragEnd, dragStart, dropOn, envToProject, envToSecret, exportData, gitProviderConnect, gitProviderDelete, gitProviderRefresh, giteaProviderPick, giteaRepoPick, gotoApp, gotoBroker, gotoCompose, gotoDatabase, gotoImport, gotoStorage, gotoTemplate, gotoType, hideSecret, hideServiceSecret, infraDownload, initAdvancedForm, initGeneralForm, initHealthcheckForm, jobDelete, jobRebuild, jobRunCancel, jobRunNow, jobToggle, jobWarm, loadData, loadMigration, nav, newJobBack, newJobCreate, newJobGitProviderPick, newJobGitRepoPick, newJobPickNoService, newJobPickProject, newJobPickService, newJobSetSourceTab, newRegistryTokenCreate, onArchiveFileChange, open, openDetail, openEditJob, openEnvText, openExportWin, openJobActions, openJobLogs, openNewJob, openNewService, openProject, openProjectWin, openService, openServicesReorder, openTokenModal, persistPrefs, pickAll, pickBroker, pickDataFile, pickDatabase, pickFile, pickNone, pickStorage, pickSuggested, pickTemplate, preDeployCheckAdd, preDeployCheckDelete, preDeployCheckMove, previewImportData, projectEnvToSecret, projectEnvToService, projectRename, projectSecretToEnv, projectSecretToService, queueCancel, queuePromote, queueTogglePause, regenWebhook, registryCloseRepo, registryGc, registryOpenRepo, registryPickerRepoPick, registryRemoveRepo, registryRemoveTag, registryRemoveToken, removeDeployment, renameService, requestProjectDeploy, reset, revealSecret, revealServiceSecret, rollbackMigration, saveAdvanced, saveCompose, saveEnvText, saveHealthcheck, saveRetention, saveServicesReorder, saveSource, searchChanged, select, serviceLogExecutionDelete, serviceLogExecutionLoadOlder, serviceLogExecutionOpen, serviceLogLoadOlder, serviceLogSelectedDelete, serviceLogToggle, serviceLogToggleAll, serviceReload, serviceRename, serviceSecretToEnv, serviceSecretToProject, serviceStop, serviceWarm, servicesReorderMove, servicesReorderStep, set, setData, setProviderTab, setServiceTab, setShared, settingsSave, showSharedDatabase, startEnvEdit, startMigration, stateGitAllowFinding, stateGitConfigure, stateGitDismissRemoval, stateGitFormLoad, stateGitOpen, stateGitResolve, stateGitResume, stateGitSyncNow, stopAll, stopAndDeleteService, stopService, submit, submitApp, submitBroker, submitCompose, submitDatabase, submitSecret, submitStorage, submitTemplate, toggleReveal, updateApply, updateCheckNow, updateRollback, uploadArchive, viewBuildLog

### sw.js — service worker do PWA Rustploy.

### wm.js
function lsGet(key)
function lsSet(key, value)
function bringToFront(layer)
function clamp(box) — Mantém a janela alcançável: ao menos a barra de título dentro da tela.
function pin(box)
function registerWindows(Alpine)

### zip.js — monta um `.zip` no navegador, sem biblioteca.
const CRC_TABLE(()
function crc32(bytes)
function dosDateTime(date) — Data/hora no formato DOS (o zip não guarda fuso).
function buildZip(files) — Monta o zip a partir de `[{ path, content }]` (texto), sem compressão, na ordem recebida.

## rustploy-daemon/crates/daemon/webui/locales/

### en.js — catálogo em inglês, por texto de origem (ver i18n.js).

### pt-BR.js — catálogo em português (pt-BR), por texto de origem (ver i18n.js).

## rustploy-daemon/crates/daemon/webui/net/

### api.js
function fetchTimeout(url, init, timeoutMs) — `fetch` com timeout cobrindo cabeçalho E corpo; o erro de timeout vira mensagem legível.
function decodeAccessKey(accessKey) — Decodifica o token de conexão em `{ address, token }` (`address` = `host:porta`), ou `null` se a str…
function encodeAccessKey(url, token) — Monta o token de conexão de um endereço (URL ou `host:porta`) + token da API — o inverso de `decodeA…
function login(baseUrl, accessKey) — `POST /api/login`: troca o token de conexão pelo token da API.
class Api
  constructor(baseUrl, token)
  headers()
  rpc(command) — Executa um Command.
  rpcChecked(command) — Como rpc(), mas trata `Response::Err { code, message }` como falha.
  downloadServiceData(serviceId, stop, encrypt) — `GET /api/services/<id>/data/export` — baixa o pacote `.rpdata` (binário).
  uploadServiceData(serviceId, file, confirm) — `POST /api/services/<id>/data/import` — envia o pacote.
  uploadArchive(serviceId, file) — `POST /api/services/<id>/archive` — corpo binário cru (não é RPC JSON).

### sse.js
function openStream(baseUrl, token, path, handlers) — Abre a stream em `path` (relativo a `baseUrl`, ex.

## rustploy-daemon/crates/daemon/webui/screens/

### dashboard.js — tela "Deployments" (view padrão do shell).
  Alpine.data("dashboard")
    get: store, rows

### deploy_engine.js — tela "Deploy Engine": fila global (um deploy por vez), execução em andamento e histórico das últimas…
  Alpine.data("deployEngine")
    dragStart(dragEvent, deploymentId)
    dragEnd()
    dropOn(targetId)
    openDetail(deploymentId)
    closeDetail()
    get: store, engine, active, detail, queued, recent, paused, uptime, successCount, failedCount, totalCount

### docker.js — tela "Docker": containers/imagens/volumes/networks do host inteiro (não só recursos geridos pelo rus…
  Alpine.data("docker")
    syncRetention()
    saveRetention()
    openTokenModal()
    closeTokenModal()
    newRegistryTokenCreate()
    get: store, containers, images, volumes, networks, repos, tags, tokens, registryHost, registryStatusLabel

### ingress.js — tela "Ingress": rotas ativas no reverse proxy (por domínio) e portas TCP de host expostas diretament…
  Alpine.data("ingress")
    get: store, pairs, routes, hostPorts

### login.js — tela de login.
  Alpine.data("login")
    submit()
    get: store, otherServerWarning

### monitoring.js — tela "Monitoring": uso de CPU/memória do host e por container.
  Alpine.data("monitoring")
    get: store, rows

### new_service.js
  Alpine.data("newService")
    ensureCatalog()
    gotoType() — ── Navegação por passos ─────────────────────────────────────────
    gotoApp()
    gotoImport()
    gotoCompose()
    gotoDatabase()
    pickDatabase(database)
    gotoBroker()
    pickBroker(broker)
    gotoStorage()
    pickStorage(storage)
    gotoTemplate()
    pickTemplate(template)
    cancel()
    reset(resetEvent) — Janela reaberta: volta ao passo 1 sem lixo do uso anterior (o catálogo dos bancos/brokers/templates …
    baseReq(kind, selectedId) — ── Submissões ────────────────────────────────────────────────────
    submitApp()
    submitCompose()
    submitDatabase()
    submitBroker()
    submitStorage()
    submitTemplate()
    get: store, filteredTemplates

### project_detail.js — projeto aberto (view=project_services no client iced): sub-abas Serviços/Variáveis/Secrets/Jobs.
function primaryContainer(service) — Container "primário" de um serviço pra exibir no card: o live, senão o primeiro em execução, senão o…
  Alpine.data("projectDetail")
    init()
    startEdit()
    cancelEdit()
    saveEdit()
    addEnvVar()
    deleteEnvVar(key)
    openEnvText()
    closeEnvText()
    saveEnvText()
    revealSecret(name) — 👁: pede o valor ao daemon (SecretReveal grava auditoria: quem, qual, quando; nunca o valor).
    hideSecret(name)
    copySecret(name)
    startEnvEdit(row, field)
    isEnvEditing(key, field)
    cancelEnvEdit()
    commitEnvEdit(row)
    projectRename(kind, from) — ── Renomear (EnvKeyRename / SecretRename) ───────────────────────── O valor nunca passa pela tela (n…
    projectMove(op, name, serviceId, doneMessage)
    targetService()
    projectEnvToSecret(key)
    projectSecretToEnv(name)
    projectSecretToService(name)
    projectEnvToService(key)
    submitSecret()
    get: store, project, services, canDelete, jobs, envVars, secrets, serviceOptions

### projects.js — tela "Projects": grid de cards + criar/editar/remover.
  Alpine.data("projects")
    submitNew()
    startEdit(row)
    cancelEdit()
    saveEdit()
    get: store, servicesCount, rows
  Alpine.data("projectWin") — Janela "Novo projeto"/"Editar projeto" (wm.js).
    init()
    submit()
    get: store, editing

### running.js — tela "Em execução": só os projetos com serviço rodando e, dentro de cada um, só os serviços rodando.
function maskWebhook(url) — `https://host/webhook/<id>/<token>` → `https://host/webhook/<id>/••••••••`.
  Alpine.data("running")
    toggleReveal(serviceId)
    get: store, projects, totalServices

### schedules.js — tela "Schedules": jobs one-shot (docker-compose) agendados ou manuais, de todos os projetos.
  Alpine.data("schedules")
    get: store, rows

### service_bundle.js
  Alpine.data("serviceExport") — ── Exportar ─────────────────────────────────────────────────────────
    init()
    reset()
    load(serviceId)
    pickSuggested()
    pickAll()
    pickNone()
    generate() — Pede o pacote ao daemon com as escolhas da tela.
    download()
    copy()
    close()
    get: store, projectVars, pickedKeys
  Alpine.data("serviceImport") — ── Importar ─────────────────────────────────────────────────────────
    pickFile(changeEvent)
    clear()
    nonEmpty(map)
    buildReq(dryRun)
    analyze(first) — Pré-visualização (nada é gravado).
    chooseEnv(key, choice) — Sobrescrever mexe no projeto e vale para os outros serviços dele.
    stateLabel(state)
    choiceLabel(statusEntry, choice)
    choicesFor(statusEntry)
    create()
    get: store, hasMissing, canCreate

### service_detail.js — detalhe de um serviço.
  Alpine.data("serviceDetail")
    init() — `x-show` mantém este componente montado por toda a sessão — abrir um serviço não recria o Alpine.dat…
    loadWebhook()
    regenWebhook()
    loadData()
    exportData()
    pickDataFile(event)
    previewImportData() — 1º passo: o daemon confere o pacote e diz o que será substituído (nada muda).
    confirmImportData() — 2º passo: confirmado, envia de novo e aplica.
    loadMigration()
    startMigration()
    rollbackMigration()
    discardOldDatabase()
    loadSharedState()
    setShared(on)
    showSharedDatabase(database)
    createSharedDatabase()
    deleteSharedDatabase(database)
    loadConnectionUrl()
    initGeneralForm()
    registryPickerLoad(image) — Porta de registry_picker_load (GUI): separa `host/repo:tag`; o host é descartado.
    registryPickerRepoPick(repo)
    registryPickerFetchTags()
    renameService() — Unicidade dentro do projeto é checada no daemon (a mensagem volta no toast).
    saveCompose()
    cancelCompose()
    setProviderTab(tab)
    giteaProviderPick(providerId)
    giteaRepoPick(fullName)
    saveSource() — Porta de handlers/services.luau::gen_save — mesma heurística looksLikeGitUrl decide Git vs Registry …
    onArchiveFileChange(event)
    uploadArchive()
    copyToClipboard(text)
    addEnvVar()
    loadServiceSecrets() — Nomes dos secrets do serviço aberto (`SecretList` com `service_id`).
    revealServiceSecret(name) — 👁: pede o valor ao daemon (SecretReveal grava auditoria: quem, qual, quando; nunca o valor).
    hideServiceSecret(name)
    copyServiceSecret(name)
    addServiceSecret() — Cria (ou sobrescreve) um secret do serviço; o novo valor vale no próximo deploy.
    deleteServiceSecret(name)
    envToSecret(key) — "🔒" numa variável em claro: o valor passa a ser guardado cifrado e a variável vira uma referência ao…
    startEnvEdit(row, field)
    isEnvEditing(key, field)
    cancelEnvEdit()
    commitEnvEdit(row)
    serviceRename(kind, from) — ✏️: renomeia a chave de uma variável (`kind` "env") ou o nome de um secret ("secret") DESTE serviço,…
    serviceMove(op, name, doneMessage)
    serviceSecretToEnv(name) — 🔓: volta a ser variável em claro (o valor deixa de ser cifrado).
    serviceSecretToProject(name) — ↑: do serviço para o projeto (todos os serviços do projeto passam a enxergar).
    envToProject(key)
    deleteEnvVar(key)
    openEnvText()
    closeEnvText()
    saveEnvText()
    addDomain()
    deleteDomain(domain)
    initHealthcheckForm() — Popula o form local a partir do spec atual — chamado ao abrir a aba (o form é editável, não reativo …
    saveHealthcheck()
    initAdvancedForm()
    saveAdvanced()
    dependencyAdd()
    dependencyDelete(dependencyId)
    preDeployCheckAdd()
    preDeployCheckDelete(jobId)
    preDeployCheckMove(jobId, delta) — Sem drag-and-drop na web UI (sem lib de DnD): reordena com botões mover-pra-cima/baixo — mesmo resul…
    viewBuildLog(deploymentId)
    closeBuildLog()
    abortDeployment(deploymentId)
    removeDeployment(deploymentId)
    get: store, webhookSupported, dataIsDatabase, canMigrate, migrationSteps, canShare, isShared, svc, projectName, statusLabel, statusKind, warm, warmLabel, warmKind, sourceText, isCompose, registryPickerRepos, registryLoopback, registryPickerImage, renameNote, liveContainers, connectionInfo, envVars, domains, runArgsText, projectServicesForDeps, dependencyRows, dependencyOptions, preDeployQueueIds, preDeployChecks, preDeployAvailableJobs, deployments

### settings.js — tela "Settings": Web Server / Git / Sync / Manutenção / Updates.
  Alpine.data("settings")
    get: store, gitProviderRows
