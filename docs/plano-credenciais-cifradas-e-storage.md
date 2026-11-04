# Credenciais cifradas por padrão, renomear chaves e o serviço Storage

> **Fases A–G feitas em 2026-11-03/04; a Fase H (uma branch só, com `watch_paths`) está só PROJETADA.** Lançadas até a **release v0.1.38** (daemon 0.1.32, gui 0.1.39). O daemon referencia o `shared` 0.16.0 por `path` — **não foi ao crates.io** (decisão do dono). **Verificação:** 460+ testes do daemon, 46 do shared, 406 testes Luau (`glacier test`) e uma verificação ponta a ponta num **daemon local real** (44 checagens: o agente via `/mcp` nunca vê a senha, o round-trip `ServiceGet`→`ServiceUpdate` não corrompe valores, rename e proibições) mais a **webui no Chrome** (criar Storage pela tela, ✏️ renomeando de verdade). **GUI nativa verificada depois** (Xvfb): wizard com Storage, ✏️ nas 4 listas, aviso de nome igual — e a verificação achou o ✏️ do serviço na linha errada e o campo de senha inútil de Kafka/NATS (corrigidos).
> Plano escrito em 2026-11-03 depois de ler o código
> (daemon 0.1.28, shared 0.13/0.14). Nada aqui foi alterado ainda; este documento existe para
> você aprovar o rumo antes. O cabeçalho dirá quando cada fase for feita.

## 0. Em duas frases

Hoje o assistente (agente via MCP) consegue **ver senhas** de bancos e brokers que o próprio
rustploy cria, e o catálogo de "Novo serviço" não tem Storage. Este plano faz o rustploy
**nascer com as senhas já cifradas**, impede que o agente as leia, deixa você **renomear chaves**
de variáveis e de secrets sem tocar no valor, e cria o **Storage (RustFS e MinIO)** — para que o
agente consiga montar a infra de um projeto inteiro (como o Chiquitos) sem nunca ver uma senha.

## 1. Por que isto existe (o caso que motivou)

No projeto Chiquitos o código do `chickie` lê `RABBITMQ_PASS`, mas o broker criado pelo wizard
usa `RABBITMQ_DEFAULT_PASS`. Para alinhar os dois sem copiar a senha de um lado para o outro, o
agente precisaria **ler a senha** (proibido pelo que você quer) ou **você** teria de redigitar.
Com renomear-chave e secrets que nascem cifrados, o agente só mexe em **nomes e referências**;
o valor nunca passa por ele.

Além disso, ao montar o plano o agente já viu uma senha de banco em claro: o
`ServiceConnectionInfo` devolve a `internal_url` com a senha. Isso é o tipo de vazamento que
queremos fechar.

## 2. Decisões já tomadas (por você)

1. **Agente nunca recebe o valor de um secret.** Bloquear `SecretReveal` **e** mascarar a senha
   nas respostas que a carregam. (`SecretReveal` já está bloqueado hoje — ver §3.)
2. **O Storage terá RustFS e MinIO**, com as credenciais geradas já cifradas, na GUI (glacier),
   na webui e no daemon.
3. **As chaves S3 vêm do próprio Storage**, não de um provedor externo — então não dependem de
   você gerar token em serviço de terceiros. (SMTP e Asaas continuam sendo seus.)

## 3. O que existe hoje (verificado no código)

| Fato | Onde |
|---|---|
| O wizard grava a senha do banco/broker como `EnvVarValue::Plain` | `features/wizard.rs` — `db_env_vars`, `broker_env_vars` |
| A senha é gerada por `token_urlsafe`: splitmix64 semeado pelo relógio — **não é aleatório criptográfico** | `features/wizard.rs:179` |
| `SecretReveal` **já é proibido ao agente** | `api/agent/tools.rs` — `proibido_ao_agente` |
| `ServiceConnectionInfo` devolve `internal_url` **com a senha** e o agente a recebe | `api/handlers/services/service_connection_info.rs` |
| `ServiceGet` devolve os valores `Plain` das envs (inclusive as senhas do wizard) | `Command::ServiceGet` |
| **Secrets de serviço existem** (`SecretSet.service_id`), além dos de projeto | `shared/src/protocol/command.rs:200` |
| `SecretMove` com `SecretToEnv` **transforma secret em `Plain`** (o agente poderia reabrir o vazamento) | `command.rs:19`, `api/handlers/secrets/secret_move.rs` |
| `SecretMove` com `EnvToSecret` **já converte env em secret sem o valor passar pelo chamador** | idem |
| `InfraAudit` já lista o que parece segredo em claro, sem valores | `Command::InfraAudit` |
| Não há comando para renomear uma chave de env nem um secret | — |
| MinIO e RustFS existem só como **template** (blueprints), não no wizard | `templates/blueprints/{minio,rustfs}` |

Duas correções a afirmações anteriores: (a) existem secrets de serviço; (b) o `SecretReveal`
já estava bloqueado ao agente — o buraco real é o `ServiceConnectionInfo` e os `Plain`.

## 4. As fases

Ordem pensada para que cada fase entregue valor sozinha e a mais barata (e mais urgente) venha
primeiro.

### Fase A — Fechar os vazamentos para o agente

> **Implementada (2026-11-03)** em `api/agent/redigir.rs` + `tools.rs` (`rpc`, `snapshot`, `proibido_ao_agente`, `restaurar_mascaras`), com 8 testes e nota no `manual.md`. Além do previsto: como o `ServiceUpdate`/`ProjectEnvSet` substituem tudo, o daemon **restaura** o valor real quando o agente reenvia o `***` (senão `ServiceGet`→`ServiceUpdate` apagaria senhas). Também ficaram proibidos ao agente `RegistryTokenCreate` e `ServiceExport` com `include_values`. Falta só o efeito no servidor: precisa de release do daemon.

**Por quê:** é a promessa central ("sem vazar para o agente") e é a mais barata.

- `tools.rs`: criar `redigir_para_agente(Response) -> Response`, aplicada em `rpc` e nas
  ferramentas dedicadas antes de devolver ao agente:
  - `ConnectionInfo`: trocar a senha da `internal_url` por `***` (o `host` fica; o agente só
    precisa dele).
  - Envs `Plain` cuja **chave** pareça segredo (`PASSWORD`, `PASS`, `SECRET`, `TOKEN`, `KEY`,
    `_URL` com credencial embutida) → `***`. É uma rede de segurança, **não** a solução: a
    solução real é a Fase B (a senha deixa de ser `Plain`).
- `proibido_ao_agente`: acrescentar `SecretMove` com `op = SecretToEnv` (ele cria `Plain`).
- Revisar o que mais carrega valor para o agente e decidir caso a caso: `ManifestExport*`,
  `ServiceExport`, `EnvBackupList/Restore`, `RegistryTokenCreate` (devolve token uma vez).
- Testes: um por comando, afirmando que o valor sensível **não** aparece na resposta do `rpc`,
  mas continua aparecendo para a GUI/webui (que não passam por `tools.rs`).

**Limite honesto:** nada impede o agente de criar um serviço que imprima suas envs no log e ler
com `LogsGet`. O MCP usa um token de administrador; isto é redução de risco por engano, não um
cofre contra um agente hostil. Fica registrado na doc do agente (`manual.md`).

### Fase B — Senhas que nascem como secret, geradas com aleatoriedade de verdade

> **Implementada (2026-11-03).** `features/wizard.rs`: `generate_password` (`/dev/urandom`, amostragem por rejeição, 24 caracteres) e `completar_senhas` (campo vazio = o daemon gera; digitado é respeitado; Kafka/NATS não ganham senha). **Descoberta:** quem gerava a senha era o *cliente* (GUI com `math.random(os.time())`; a webui exigia digitar) — agora os dois mandam vazio e deixam o daemon gerar. A passagem para secret de serviço **já existia** (`api/handlers/services/wizard.rs::secure_credentials`, R5 do plano de backup) e foi reaproveitada. Pendente da B: o botão "ver valor" logo após criar (hoje a pessoa vê no secret do serviço, com `SecretReveal`).

**Por quê:** hoje cada banco/broker novo já nasce com a senha exposta e fraca.

- Gerador: trocar `token_urlsafe` por bytes de `/dev/urandom` (já é o padrão do repo em
  `generate_secret`), mantendo o alfabeto URL-safe.
- `wizard.rs`: ao criar Database/Broker, o daemon (1) gera a senha se o formulário veio vazio,
  (2) grava um **secret de serviço** (ex.: `POSTGRES_PASSWORD`) e (3) põe na env
  `EnvVarValue::Secret("POSTGRES_PASSWORD")`. A senha nunca vai para a resposta do
  `WizardCreate` nem para o log.
- A GUI/webui deixam de mostrar a senha gerada; mostram "gerada e cifrada" e, para a pessoa, o
  botão "Ver valor" (que usa `SecretReveal`, permitido a humanos).
- **A verificar na implementação:** que o `docker compose` do banco (que usa `${POSTGRES_PASSWORD}`)
  recebe o valor resolvido do secret — o `env_resolve` já resolve `Secret` no deploy, mas é preciso
  confirmar o caminho do compose com um deploy real.
- **Migração do que já existe:** `InfraAudit` lista os `Plain` suspeitos e `SecretMove EnvToSecret`
  os converte **sem o valor passar pelo agente**. No Chiquitos isso vale para o `db`, o `broker`
  e o `cache`. (Provavelmente nem precisa de código novo.)

### Fase C — Renomear chaves

> **Implementada (2026-11-03).** `shared` 0.16.0 (`EnvKeyRename`, `SecretRename`); `platform/secrets.rs` (`rename_env_key`, `rename_secret`, 6 testes); handlers em `api/handlers/secrets/`; GUI (✏️ + mini-formulário nos 4 lugares) e webui (✏️ + `prompt`). `SecretRename` de projeto **respeita o sombreamento**: serviço/job com secret próprio de mesmo nome não é reapontado. Liberados ao agente (não revelam valor).

**Por quê:** é o que faltou no caso `RABBITMQ_DEFAULT_PASS` × `RABBITMQ_PASS`.

Dois comandos novos em `shared` (**mudança de protocolo → bump do shared**):

- `EnvKeyRename { project_id, service_id: Option, from, to }` — troca só a **chave** de uma env do
  projeto ou do serviço, preservando o valor (`Plain` ou `Secret`). Falha se `to` já existe.
- `SecretRename { project_id, service_id: Option, from, to }` — renomeia o secret **e** atualiza
  todas as referências `Secret(from)` no escopo (envs do projeto e dos serviços). Falha se `to`
  existe. Esta é a peça que não dá para fazer por fora, porque exige ler/regravar o valor cifrado.

Observação: renomear a **chave** de uma env `Secret` não precisa mexer no secret (a chave da env e
o nome do secret são independentes: `RABBITMQ_PASS = Secret("RABBITMQ_DEFAULT_PASS")` já é
possível hoje). Por isso `EnvKeyRename` resolve a maioria dos casos e `SecretRename` é para
arrumar o nome do próprio secret.

- Daemon: handlers em `api/handlers/secrets/` e `.../projects`; permitidos ao agente (não revelam
  valor). Cuidado com o plano `plano-yaml-fonte-da-verdade`: a renomeação tem de sair no YAML de
  estado sincronizado em git e nos `ENC[age:…]`.
- GUI (glacier): botão "renomear" nas listas de variáveis e de secrets (`.gv` + `.luau`, com
  entrada na lista `busy_actions`). webui: o mesmo. Strings nos locales dos dois.

### Fase D — Novo serviço: Storage (RustFS e MinIO)

> **Implementada (2026-11-03), exceto o bucket.** `StorageKind` + `storage_spec`/`storage_compose` em `features/wizard.rs`; `WizardCreate` com `kind: "storage"`; catálogo com `storages` (shared 0.16.0, `#[serde(default)]`); painel "Em execução" com a categoria `storage`; GUI (passo `pick_storage`, reaproveita o `db_form`) e webui (`gotoStorage`/`storage_form`). Credenciais geradas no daemon e gravadas como secret de serviço (teste: a senha não aparece na resposta nem no YAML). **Bug achado pelo teste e corrigido:** a senha do RabbitMQ (`RABBITMQ_DEFAULT_PASS`) ficava `Plain` — a heurística de nomes sensíveis não reconhecia `_PASS`/`PWD`; agora reconhece (isso também endurece o `InfraAudit`). **Bucket resolvido sem código novo** (testado com Podman em 2026-11-03, RustFS e MinIO): um job com `trigger_service_id` = o Storage herda o ambiente já resolvido (`env_resolve::resolve_job`), então roda `aws s3 mb` com `docker.io/amazon/aws-cli` sem a chave passar pelo agente; repetir é seguro. A receita está no `manual.md`. **Achados do teste:** `minio/mc` (Docker Hub e Quay) e `minio/minio` (Docker Hub e Quay) **não são mais puxáveis** — o padrão do wizard para MinIO passou a `cgr.dev/chainguard/minio:latest` (sobe com o mesmo comando e as mesmas variáveis; credencial errada é recusada). O blueprint `minio` do catálogo de *templates* ainda aponta para `minio/minio` e está quebrado — não mexi (fora do escopo). **Ainda aberto:** chave S3 própria por app (hoje a app usaria a chave root). **Achado do teste ponta a ponta:** `WizardCreate` chamado pelo agente com `user`/`db_name` vazios criava o banco **sem usuário** (a GUI sempre mandava o padrão); o daemon agora aplica os padrões (usuário do motor, banco `app`, `rabbit`).

**Por quê:** o código do chickie precisa de um S3; hoje o único caminho é montar o template à mão.

- `shared`: `StorageKind { RustFs, MinIo }` e o que o catálogo precisa (imagem, portas 9000
  API e 9001 console, `kind_id`, rótulo). **Mudança de protocolo → bump do shared.**
- `features/wizard.rs`: `storage_catalog_json`, `storage_spec`, `storage_compose` (partindo dos
  `docker-compose.yml` dos blueprints). Credenciais geradas na Fase B:
  - RustFS: `RUSTFS_ACCESS_KEY` e `RUSTFS_SECRET_KEY` (o blueprint usa `rustfsadmin` + senha
    de 16 caracteres; passam a ser secrets de serviço, geradas aleatoriamente).
  - MinIO: `MINIO_ROOT_USER` e `MINIO_ROOT_PASSWORD`.
- `Command::WizardCatalog`/`WizardCreate`: novo tipo `Storage`.
- `ServiceConnectionInfo` para Storage: devolve `host` e `endpoint` (`http://rp_<nome>:9000`),
  **sem** as chaves (Fase A).
- GUI: nova etapa "Storage" no `new_service_window.gv/.luau` e em `home/…`; webui: o mesmo.
  Índices, i18n pt-BR/en e `make index`.
- **Aberto:** o RustFS **não cria o bucket sozinho**. Preciso decidir como criá-lo sem expor a
  chave: (a) um job/pré-deploy rodando um cliente S3 com as variáveis do próprio serviço; (b) o
  daemon assinando um `CreateBucket` (exige um cliente S3 no daemon, o que contraria a regra de
  poucas dependências). Recomendação: (a), reaproveitando Jobs/Pré-deploy.

### Fase E — Ligar a app ao Storage

O código do chickie lê `S3_ACCESS_KEY_ID`, `S3_SECRET_ACCESS_KEY`, `S3_ENDPOINT`,
`S3_BUCKET_NAME_1`, `S3_PUBLIC_URL`. Isso se resolve só com referências, sem novo código:
`S3_ACCESS_KEY_ID = Secret("RUSTFS_ACCESS_KEY")` etc. (o nome do secret não precisa ser igual à
chave da env). **Aberto:** a app usaria a chave **root** do RustFS; o ideal é uma chave própria
com permissão só no bucket — depende de o RustFS expor isso por API; não está verificado.

### Fase F — Infra do Chiquitos (operação, não é código do rustploy)

Depois de A–E, com o agente:

1. Converter o `db`, o `broker` e o `cache` para secrets (`SecretMove EnvToSecret`) ou recriá-los
   pelo wizard novo.
2. Criar o Storage (RustFS) e o bucket.
3. Alinhar as chaves das envs com `EnvKeyRename` (`RABBITMQ_PASS` etc.) e remover duplicatas.
4. Ajustar o código do `chickie` só onde as envs não bastarem (commit/push: **pergunto antes**).
5. Subir um a um: db → cache → broker → storage → scheduler → worker → api, conferindo
   `build_logs`/`list_deploys` a cada passo. Pontos em que **você** entra: SMTP e Asaas.

### Fase G — Provisionamento fora da app: `chickie-setup` e a branch `main-setup` (FEITA em 2026-11-04)

Não estava no plano original; saiu da pergunta "por que o bucket precisa de `curl`?". A api do
chickie criava o bucket e rodava as migrations na própria subida. Agora **nada disso é da api**:

- **Repo `chickie`:** crate nova `crates/setup` (`chickie-setup migrate | bucket`), só com
  `hyper`/`hmac`/`sha2` (o cliente S3 que já existia no `core`). A api, o worker e o scheduler
  não migram nem criam bucket na subida.
- **Jobs no Chiquitos:** `migrations` e `bucket` são `git_source` da branch **`main-setup`**
  (`dockerfiles/migrate.compose.yml`, `dockerfiles/bucket.compose.yml`, ambos com `build:` da
  `setup.dockerfile`) e continuam como pré-deploy da api. O `bucket` não usa mais `curl`.
- **Verificado:** `migrate` num Postgres 18 real (30 migrations, seed, idempotente), `bucket`
  contra um S3 falso (HEAD/PUT/PUT policy em SigV4, idempotente), e os dois jobs rodaram no
  daemon do Chiquitos com `exit 0`. **Redeploy da api (dep_01M6JB70…, 2026-11-04):** `Live`; os pré-deploys `migrations` (30 já aplicadas) e `bucket` (já existia, policy reaplicada) passaram, e o log da api não tem nenhuma linha de migração nem de bucket. **Ainda não verificado:** redeploy do worker e do scheduler com o código novo (as branches deles receberam o merge da `main`), e a receita de bucket nas docs (ainda ensina `curl`).
- **Custo que sobrou:** as branches `main`, `main-api`, `main-worker`, `main-scheduler` e
  `main-setup` precisam do `make sync` (merge da `main` em cada uma). É isso que a Fase H elimina.

### Fase H — Uma branch só, com `watch_paths` (ÚLTIMO PASSO; NÃO implementada)

Hoje o webhook do daemon só filtra o push pela **branch** (`public_routes.rs`); o campo
`ServiceSpec.watch_paths` é gravado mas o deploy **não o usa** (confirmado em
`referencia/manifesto.mdx`). Por isso o chickie tem uma branch por serviço e o `make sync`.

1. **Daemon:** o webhook lê `commits[].added/modified/removed` do payload (GitHub/Gitea/Gogs) e
   ignora o deploy se `watch_paths` não vazio e nenhum arquivo alterado casar. Sem `watch_paths`
   o comportamento atual não muda. Testes: casa, não casa, vários commits, payload sem lista de
   arquivos (nesse caso **faz o deploy**, para não perder um push).
2. **GUI e webui:** campo "Caminhos monitorados" no serviço Git (os dois clientes).
3. **Docs e manifesto:** deixar de dizer "ainda não usa".
4. **Os apps:** cada serviço do chickie passa a apontar para a `main`, com seu `watch_paths`
   (api: `crates/api`, `crates/core`, `dockerfiles/api.dockerfile`, `Cargo.*`; análogo para
   worker e scheduler); os jobs `migrations`/`bucket` passam para a `main`; as branches `main-*`
   e o alvo `sync` do Makefile são aposentados (arquivar, não apagar).

**Ordem:** itens 1–3 (no rustploy, com release) → item 4 (configurar os apps).

## 5. Publicação

Seguir o checklist "Publish" do `AGENTS.md`. Ordem: `rustploy-shared` (bump: Fases C e D mudam o
protocolo) → `rustploy-daemon` → `rustploy-gui`, depois os ponteiros dos submodules no agregador.
A Fase A é só daemon. Antes de publicar qualquer UI: rodar a GUI/webui de verdade (regra do
repo), não só `cargo test`.

## 6. Riscos e o que não cobre

- **O agente continua com token de administrador.** As fases fecham os caminhos óbvios de
  vazamento, não os criativos (ex.: ler env de um container via log que o próprio agente criou).
- **Rotação de credenciais:** converter uma senha `Plain` em secret não a troca. Quem viu uma
  senha em claro (inclusive o agente, antes da Fase A) deve considerá-la exposta e rotacioná-la.
  No Chiquitos isso vale para a senha do `db`.
- **Compatibilidade:** serviços já criados com `Plain` seguem funcionando; nada é migrado sem
  você pedir.

## 7. Perguntas em aberto (preciso da sua resposta antes da Fase D)

1. Bucket: seguir a recomendação (a) — criar por job/pré-deploy?
2. A rede de segurança por nome de chave (Fase A) é aceitável, sabendo que pode mascarar um valor
   inocente cuja chave tenha `KEY` no nome?
3. Quer a Fase A publicada **antes** das outras (como release própria do daemon)? Recomendo que sim.
