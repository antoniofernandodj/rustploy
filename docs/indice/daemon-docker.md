# Índice: daemon: Docker/Compose e ingress (proxy, TLS)

> Gerado por `cargo run -p indexer`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## crates/daemon/src/docker/

### compose.rs — Serviços e jobs Docker Compose: `up` de stack com a rede do projeto injetada, execução one-shot de j…
fn registry_login(token) -> Result<()> — Loga no registry embutido com o token interno `rp-internal` antes de um `docker compose up` (que pod…
fn registry_logout() -> Result<()> — Desloga do registry embutido (limpeza best-effort, chamada sempre que `registry_login` foi tentado, …
fn inject_project_network(content, network_name) -> Result<String> — Reescreve o YAML do Compose para que todo serviço entre também na rede do projeto (declarada como `e…
fn ensure_external_volumes(docker, content) -> Result<()> — Garante que todo volume declarado `external: true` no compose já exista no Docker antes do `up` — o …
fn up(docker, content, project_name, service_id, deployment_id, network_name, bus, db, env_vars, build_dir, registry_internal_token) -> Result<()> — Sobe (ou atualiza) a stack Compose de um serviço: grava `.env` e `docker-compose.yml` com a rede do …
enum JobBuildSource { Compose, Git } — Sobe um stack docker-compose UMA VEZ (job one-shot, não um serviço de vida longa): usa `--abort-on-c…
const CANCELLED_EXIT_CODE — Exit code sentinel pra "job_run cancelado pelo usuário" (`JobRunCancel`) — distinto do `-1` genérico…
fn wait_for_cancel(rx) — Espera até o sinal de cancelamento virar `true` (ou o sender ser derrubado, o que só acontece se a t…
fn run_once(source, project_name, network_name, main_service, job_id, job_run_id, bus, db, env_vars, build_dir, registry_internal_token, mirror_deployment, cancel_rx) -> Result<i32> — Executa um job uma vez: prepara o compose (colado ou do repositório git) com a rede do projeto, roda…
const COMPOSE_TAIL_MAX — Quantas linhas do próprio `docker compose` (build, pull, erros de container) o job guarda pra despej…
fn split_service_prefix(line) -> Option<(&str, &str)> — Separa o prefixo `<service>-<replica> |` que o `docker compose up` (sem `-d`) põe em cada linha de s…
fn filter_main_service_line(line, main_service) -> Option<&'a str> — Filtra uma linha bruta de `docker compose up` (sem `-d`) pra só sobrar a saída do `main_service`: se…
fn clean_compose_line(line) -> Option<String> — Limpa uma linha de controle do compose pra caber no log do job: fica só o último quadro de um spinne…
fn record_job_line(bus, db, job_id, job_run_id, mirror, stream, line) — Grava uma linha no log do job (tabela `job_log` + evento ao vivo) e, se o job espelha um deployment,…
fn read_compose_output(reader, stream, main_service, bus, db, job_id, job_run_id, mirror, compose_tail) — Lê um dos pipes do `docker compose up` até fechar.
fn run_once_up(project_name, compose_rel_path, main_service, job_id, job_run_id, bus, db, build_dir, mirror_deployment, cancel_rx) -> Result<i32> — Roda o `docker compose up` de um job até o `main_service` sair, transmitindo stdout/stderr para o lo…
fn down(content, project_name, _network_name, env_vars) -> Result<()>
const PROJECT_NET_ALIAS
(11 testes)

### containers.rs — Containers de serviços Application: nomes (live, staging, réplicas, legado), busca por label, criaçã…
fn with_replica_suffix(base, idx) -> String
fn staging_name(service_id, service_name, deployment_id_short) -> String
fn replica_staging_name(service_id, service_name, dep_short, idx) -> String
fn replica_live_name(service_id, service_name, idx) -> String
fn legacy_replica_staging_name(service_name, dep_short, idx) -> String — Nome de staging no formato legado (`rp_<safe>_staging_<dep>[_r<i>]`).
fn find_staging(docker, service_id, service_name, dep_short, idx) -> Result<Option<String>> — Staging da réplica `idx` deste deploy: no nome atual ou, para um deploy iniciado antes da atualizaçã…
fn replica_index(labels, name) -> u32 — Índice de réplica de um container: o label `rustploy.replica` ou, em container de antes do label, o …
fn is_staging_name(name) -> bool — É um staging (nome termina em `_staging_<dep>` ou `_staging_<dep>_r<i>`)? Staging promovido é renome…
struct LiveReplica { id, name, replica, running } — Réplica live de um serviço Application, achada por label.
fn find_live_replicas(docker, service_id) -> Result<Vec<LiveReplica>> — Todas as réplicas live (não-staging) de um serviço, **pelo label** `rustploy.service_id` — não pelo …
fn live_replica_ips(docker, service_id, network, replicas) -> Vec<String> — IPs, na rede `network`, das réplicas live `0..replicas` de um serviço — uma por réplica, a rodando q…
fn attach_network_alias(docker, container_id, network, alias) -> Result<()> — Dá ao container o alias `alias` na rede `network`, reconectando-o (`disconnect` + `connect`): o Dock…
fn create_staging(docker, spec, image, service_id, deployment_id, network_id, resolved_env, container_name, replica) -> Result<String> — Cria o container de staging de uma réplica já na rede do projeto (`network_mode` e `NetworkingConfig…
fn start(docker, container_id) -> Result<()>
fn is_not_found(e) -> bool — Retorna `true` se o erro do bollard for um 404 (container inexistente).
fn stop_graceful(docker, container_id, timeout) -> Result<()>
fn rename(docker, container_id, new_name) -> Result<()>
fn remove(docker, container_id) -> Result<()>
fn inspect(docker, container_id)
fn get_container_ip(docker, container_id, network_name) -> Result<String> — IP do container na rede dada, lido via `docker network inspect` (o IP do `container inspect` vem vaz…
fn get_container_logs(docker, container_id, tail) -> Vec<String> — Returns the last `tail` lines of stdout+stderr from a container (best-effort).
fn find_all_by_service_id(docker, service_id) -> Result<Vec<String>>
struct ManagedContainer { id, name, state } — Container gerenciado pelo rustploy, no formato leve que o GUI exibe (id + nome + estado).
struct ContainerIndex { by_service_id, by_compose_project } — Índice de containers do host resolvido numa única listagem, com duas chaves: por `rustploy.service_i…
impl ContainerIndex
  fn for_service(service) -> Vec<ManagedContainer> — Containers de um serviço: pelos labels `rustploy.service_id`, ou — quando vazio (serviço Compose) — …
fn list_compose_projects(docker) — Stacks Compose que existem no host (`com.docker.compose.project`, com os containers vivos ou parados…
fn index_containers(docker) -> ContainerIndex — Lista **todos** os containers do host numa única chamada e os indexa por service_id e por projeto Co…
fn find_old_containers(docker, service_id, exclude_deployment_id) -> Result<Vec<String>> — Returns container IDs for a service excluding those from the given deployment.
fn find_by_name(docker, name) -> Result<Option<String>> — TODO: sem uso desde a checagem de dono.
fn label_owned_by(label, service_id) -> bool — O label `rustploy.service_id` de um container identifica `service_id`?
fn name_owner(docker, name) -> Result<Option<(String, Option<String>)>> — Quem ocupa o nome `name` no host: `None` se não há container com esse nome; senão `(container_id, ru…
fn find_owned_by_name(docker, name, service_id) -> Result<Option<String>> — Como [`find_by_name`], mas só devolve o container se ele pertencer a `service_id` (label `rustploy.s…
fn list_running_by_prefix(docker, prefix) -> Result<Vec<(String, String)>> — Containers **rodando** cujo nome começa com `prefix`, como `(id, nome)`, ordenados por nome.
fn exposed_tcp_ports(docker, container_id) -> Vec<u16> — Portas TCP que o container declara expor (`EXPOSE` da imagem + `expose:`/`ports:` do compose).
fn match_ingress_service(candidates, prefix, svc) -> Option<usize> — Índice do candidato cujo nome é o do serviço `svc` dentro do compose.
fn match_exposed_port(exposed, want_ports) -> Option<usize> — Índice do primeiro candidato que expõe alguma das portas pedidas.
fn find_compose_ingress_container(docker, project_name, ingress_service, want_ports) -> Result<Option<String>> — Escolhe qual container de uma stack Compose recebe o tráfego do ingress.
(13 testes)

### images.rs — Imagens: pull (com credenciais) e build a partir de um contexto empacotado em tar, com progresso no …
fn pull(docker, image, service_id, deployment_id, bus, db, credentials) -> Result<()>
fn _exists(docker, image) -> bool
fn build(docker, db, context_path, dockerfile, tag, service_id, deployment_id, bus) -> Result<()>
fn create_tar_gz(context_path, _dockerfile) -> Result<Vec<u8>>
fn append_dir_filtered(tar, src, prefix) -> Result<()> — Adiciona `src` ao tar sob `prefix`, pulando entradas cujo nome seja `.git`.
fn _prune_unused(docker, keep_tags) -> Result<()>

### mod.rs — Cliente Docker (`DockerClient`, via bollard) e os submódulos por recurso.
struct DockerClient { inner, _socket_path }
impl DockerClient
  fn connect(socket_path) -> Result<Self>
  fn ping() -> Result<()>

### networks.rs — Rede Docker por projeto: nome e criação sob demanda.
fn legacy_project_net_for(project_id) -> String — Nome de rede no formato **legado** (`rp_net_` + 8 primeiros chars do ID), derivado a cada uso e sem …
fn id_short(id) -> &str
fn ensure_project_network(docker, name) -> Result<String> — Garante que a rede `name` existe (cria uma bridge se não) e devolve o id.
fn _remove_project_network(docker, name) -> Result<()>
fn _connect_container(docker, network_name, container_id) -> Result<()>
fn _disconnect_container(docker, network_name, container_id) -> Result<()>

## crates/daemon/src/ingress/

### mod.rs — Ingress: proxy reverso HTTP/HTTPS embutido, tabela de rotas e TLS/ACME.

### proxy.rs — Proxy reverso HTTP/1.1 embutido, construído sobre hyper.
type ProxyBody = BoxBody<Bytes, hyper::Error>
fn start_proxy(routes, http_port, https_port, tls) — Inicia o proxy de ingress.
fn serve_http_connection(stream, routes, challenges, redirect_https)
fn handle_http(req, routes, challenges, redirect_https)
fn start_https_listener(routes, acceptor, addr)
fn serve_https_connection(stream, routes)
fn handle(req, routes, is_tls)
fn forward(req, backend_addr, is_tls)
fn serve_port_proxy(port, backend) — Listener TCP dedicado para uma porta específica de serviço.
fn serve_port_connection(inbound, backend)
fn empty_body() -> ProxyBody
fn text_body(s) -> ProxyBody
fn status_response(status) -> Response<ProxyBody>

### router.rs — Tabela de rotas do ingress (domínio → backends, porta → backends) com round-robin, lida sem lock pel…
struct RouteEntry { domain, backends, cursor, service_id }
impl RouteEntry
  fn next_backend() -> Option<String>
struct RouteTable { routes }
impl RouteTable
  fn get(domain) -> Option<&RouteEntry>
fn new_entry(table, domain, backends, service_id) -> RouteEntry — Entrada nova para `domain`, reaproveitando o cursor de round-robin da tabela atual para que a posiçã…
type RouteHandle = Arc<ArcSwap<RouteTable>> — Shared handle to the live route table, readable lock-free from the proxy thread.
struct PortBackends { addrs, cursor }
impl PortBackends
  fn new(addrs) -> Self
  fn next() -> Option<String>
type PortBackend = Arc<ArcSwap<Option<PortBackends>>> — Backend(s) atual para um listener de porta específica.
struct IngressController { table, port_backends }
impl IngressController
  fn new() -> Self
  fn upsert_route(domain, backends, service_id)
  fn remove_route(domain)
  fn register_domains(spec, ips, service_id) — Registra as rotas de domínio de um serviço e poda as órfãs dele.
  fn remove_domains(spec) — Remove todas as rotas de domínio do serviço (parada/remoção/reconcile).
  fn _lookup(domain) -> Option<RouteEntry>
  fn snapshot() -> shared::IngressSnapshot — Foto da tabela de rotas viva, para `Command::IngressRoutes`.
  fn table_handle() -> RouteHandle
  fn upsert_port_route(host_port, backends) — Aponta `host_port` para os `backends` fornecidos.
  fn remove_port_route(host_port) — Remove o roteamento de `host_port` (conexões novas são recusadas com reset).
impl Default for IngressController
(3 testes)

### tls.rs — Certificados TLS: resolução por SNI, emissão e renovação ACME (Let's Encrypt) e os desafios HTTP-01.
type ChallengeStore = Arc<Mutex<HashMap<String, String>>> — token → key_authorization: compartilhado com o handler HTTP para servir challenges.
struct SniResolver { certs }
impl ResolvesServerCert for SniResolver: resolve
struct TlsManager { cert_dir, challenges, resolver, server_config, acme_config }
impl TlsManager
  fn new(cert_dir, acme_config) -> Result<Self>
  fn tls_acceptor() -> tokio_rustls::TlsAcceptor — Retorna um TlsAcceptor que reutiliza o ServerConfig imutável.
  fn enable_acme(email) — Ativa ACME dinamicamente — sem precisar reiniciar o daemon.
  fn disable_acme() — Desativa ACME (chamado quando o e-mail é removido).
  fn ensure_cert(domain) -> Result<()> — Garante que exista um certificado válido para `domain`.
  fn renew_expiring() -> Result<Vec<String>> — Renova certificados que expiram em menos de 30 dias.
  fn cert_is_valid(domain) -> bool
  fn cert_file_expires_soon(path) -> bool
  fn load_or_create_account(email, directory) -> Result<Account>
  fn save_cert(domain, cert_pem, key_pem) -> Result<()>
  fn parse_certified_key(cert_pem, key_pem) -> Result<Arc<CertifiedKey>>
  fn load_all_from_disk()
  fn remove_challenges(pending)
impl std::fmt::Debug for SniResolver
