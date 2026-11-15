# Índice: update-core — atualização comum ao daemon e ao app

> Gerado por `make index`; não editar à mão. Sem números de linha:
> `grep -n "nome" <dir><arquivo>` dá a linha. Cada item: `nome(params) — doc`.
> Rust: métodos indentados sob `impl Tipo`; `struct`/`enum` listam campos/variantes;
> `impl A, B for T` = impls de traits comuns (métodos omitidos).

## rustploy-shared/update-core/src/

### dpkg_version.rs — Comparação de versões de **pacote Debian** (`1:0.1.10-1`), pelo algoritmo da política Debian (o mesm…
struct DpkgVersionError { input }
struct Parts { epoch, upstream, revision }
fn split(version) -> Result<Parts<'_>, DpkgVersionError>
fn order(byte) -> i32 — Peso de um caractere que não é dígito: `~` vem antes de tudo (até do fim da string), letras vêm ante…
fn compare_part(left, right) -> Ordering
fn compare(left, right) -> Result<Ordering, DpkgVersionError> — Compara duas versões de pacote Debian (época, depois upstream, depois revisão).
fn is_newer(candidate, installed) -> Result<bool, DpkgVersionError> — `true` se `candidate` é **estritamente** mais nova que `installed`.
impl fmt::Display, std::error::Error for DpkgVersionError
(5 testes)

### installer.rs — A instalação de um pacote assinado, em duas fases:
struct Paths { allowed_dir, state_dir, managed_binaries } — Onde o updater lê e escreve.
impl Paths
  fn staging_dir() -> PathBuf
  fn backup_dir() -> PathBuf
  fn outcome_file() -> PathBuf
  fn lock_file() -> PathBuf
struct Settings { package, service, trusted_keys, max_deb_bytes, poll_interval, stable_polls, max_polls }
struct Updater { paths, settings, system }
struct RollbackPlan { _lock, previous_version } — Um rollback já aprovado.
struct Prepared { _lock, staged_deb, staged_signature, facts } — Um pacote já aprovado.
impl Updater<'_, S>
  fn prepare(deb, signature_path) -> Result<Prepared, String>
  fn acquire_lock() -> Result<fs::File, String>
  fn verify_and_check(deb, signature_path, known_version)
  fn installed_version() -> Result<Option<String>, String>
  fn execute(prepared) -> UpdateOutcome
  fn prepare_rollback() -> Result<RollbackPlan, String> — Fase 1 do rollback: só **recusa** (trava e backup existente), sem mexer em nada.
  fn execute_rollback(plan) -> UpdateOutcome — Fase 2: volta os binários para os do último backup (o único caminho que aceita uma versão menor), re…
  fn rollback() -> Result<UpdateOutcome, String> — As duas fases juntas (nos testes não há cliente esperando uma resposta no meio).
  fn finish(prepared, version, kind, message) -> UpdateOutcome
  fn record(version, kind, message) — Grava o resultado.
  fn run_checked(program, args, envs) -> Result<CommandOutput, String>
  fn backup_binaries() -> Result<(), String>
  fn restore_and_restart() -> Result<(), String>
  fn service_name() -> &str
  fn wait_until_stable() -> bool — `true` se o serviço ficou `active` por `stable_polls` checagens seguidas.
fn backup_file(backup_dir, binary) -> Option<PathBuf>
fn replace_atomically(source, target) -> io::Result<()> — Troca `target` por uma cópia de `source` sem nunca deixá-lo pela metade: copia para um nome ao lado …
fn restore_note(restored) -> String
fn tail(output) -> String — Últimas linhas do que o comando disse, para a mensagem que a pessoa vai ler.
const MAX_SIGNATURE_BYTES
(12 testes)

### lib.rs — Lógica de atualização comum ao daemon e ao app: um só lugar decide se uma versão é mais nova que out…

### outcome.rs — O resultado da última atualização, gravado pelo updater (root) e lido pelo daemon depois de reinicia…
enum OutcomeKind { InProgress, Installed, Failed, RolledBack }
struct UpdateOutcome { package, version, kind, message, at_unix }
impl UpdateOutcome
  fn new(package, version, kind, message) -> Self
fn write_outcome(path, outcome) -> io::Result<()> — Grava de forma atômica (arquivo temporário + `rename`) e legível por todos: o daemon (usuário `rustp…
fn read_outcome(path) -> io::Result<Option<UpdateOutcome>> — `None` se ainda não houve nenhuma atualização; erro se o arquivo existir e estiver ilegível.
(3 testes)

### package.rs — A decisão "este pacote assinado pode ser instalado aqui?", como função pura: cruza o que a **assinat…
struct PackageFacts { package, version, architecture } — Campos lidos do `.deb` com `dpkg-deb -f <arquivo> Package Version Architecture`.
struct Expectations { package, machine_architecture, installed_version } — O que este updater aceita instalar.
enum PackageError { WrongPackage, ClaimsMismatch, WrongArchitecture, NotNewer, BadVersion, UnreadableFields }
fn parse_dpkg_fields(output) -> Result<PackageFacts, PackageError> — Lê a saída de `dpkg-deb -f <arquivo> Package Version Architecture` (linhas `Campo: valor`, em qualqu…
fn check_package(claims, facts, expectations) -> Result<(), PackageError> — Recusa se qualquer regra falhar.
impl fmt::Display, std::error::Error for PackageError
(7 testes)

### release.rs — Os arquivos de uma release do GitHub e a escolha do pacote certo para esta máquina.
struct ReleaseAsset { name, url } — Um arquivo anexado à release (`rustployd_0.1.10-1_amd64.deb`, o `.minisig`…).
struct ReleaseInfo { tag, html_url, assets } — O que interessa do JSON de uma release.
struct ReleasePayload { tag_name, html_url, assets }
struct AssetPayload { name, browser_download_url }
fn parse_release(json) -> Result<ReleaseInfo, String> — Lê o JSON de `GET /repos/{repo}/releases/latest`.
fn debian_architecture() -> Result<&'static str, String> — Arquitetura no vocabulário do Debian, a que aparece no nome do `.deb`.
fn debian_architecture_of(rust_architecture) -> Result<&'static str, String>
fn select_package_assets(assets, repo, package, architecture, tag) — O `.deb` de `package` para `architecture` e a `.minisig` dele, **dentro do repositório dado** (uma U…
struct PackageVersion { full, upstream } — A versão de um pacote, como a release a anuncia no nome do arquivo.
fn package_version(assets, repo, package, architecture) -> Option<PackageVersion> — A versão do `.deb` de `package` para `architecture` que a release traz, lida do **nome do arquivo** …
(10 testes)

### signature.rs — Verificação da assinatura minisign de um pacote e leitura do comentário confiável.
const TRUSTED_PUBLIC_KEYS — Chaves públicas em que os updaters confiam; qualquer uma delas basta.
struct SignedClaims { package, version, arch } — O que a assinatura afirma sobre o arquivo assinado.
enum SignatureError { NoTrustedKeys, BadTrustedKey, MalformedSignature, NotTrusted, MissingClaims }
fn verify(data, signature_text, public_keys) -> Result<SignedClaims, SignatureError> — Confere `data` contra `signature_text` (o conteúdo do `.minisig`) usando as chaves dadas e devolve o…
fn parse_claims(trusted_comment) -> Result<SignedClaims, SignatureError>
impl fmt::Display, std::error::Error for SignatureError
(10 testes)

### staging.rs — Cópia segura do pacote para um diretório que só o root controla.
enum StagingError { OutsideAllowedDir, BadFileName, NotRegularFile, TooLarge, Io }
fn is_plain_file_name(name) -> bool
fn stage_file(source, allowed_dir, staging_dir, max_bytes) -> Result<PathBuf, StagingError> — Copia `source` para `staging_dir` (criado `0700`, cópia `0600`) e devolve o caminho da cópia.
impl fmt::Display, std::error::Error, From<io::Error> for StagingError
(9 testes)

### system.rs — Tudo que o updater pede ao sistema (rodar comando, esperar) passa por aqui, para os testes trocarem …
struct CommandOutput { success, stdout, stderr }
trait System
  fn run(program, args, envs) -> io::Result<CommandOutput> — Roda `program` com `args`, num ambiente limpo (só `PATH`, `LANG` e `envs`).
  fn sleep(duration)
struct RealSystem
impl System for RealSystem: run, sleep

### version.rs — Comparação de versões de release (semver), tolerante ao `v` das tags.
struct VersionError { input } — A entrada não é uma versão semver (depois de tirar espaços e o `v`/`V` inicial).
fn parse_version(text) -> Result<Version, VersionError> — Lê `0.1.15`, `v0.1.15` ou `V0.1.15` (com ou sem espaços ao redor).
fn is_newer(candidate, installed) -> Result<bool, VersionError> — `true` se `candidate` é **estritamente** mais nova que `installed`.
impl fmt::Display, std::error::Error for VersionError
(7 testes)
