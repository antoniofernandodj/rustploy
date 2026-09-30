# Plano: banco compartilhado entre projetos + migração dos bancos antigos

> **Status:** em implementação — ver `docs/plano-acao-banco-compartilhado.md` §8
> para o que já entrou (Fase 1 = §3 feita em 2026-09-30; o resto ainda é proposta).
>
> **Revisão 2** (mesmo dia), por diretriz do usuário:
> 1. a connection string gerada vai para **env var comum (Plain)**, não para
>    secret — e passa a ser montada num lugar só, no Rust, porque a que a
>    interface mostra hoje não serve como `DATABASE_URL` (ver §3);
> 2. análise de **choque de nome/alias DNS** ao integrar redes de projetos
>    diferentes — mudou a topologia de rede da revisão 1 (ver §2);
> 3. suporte a **Postgres, MySQL/MariaDB e MongoDB**, não só Postgres (ver §4).

## 1. O problema, em termos simples

Hoje o servidor tem 3 projetos, cada um com seu próprio banco rodando num
serviço Compose dentro do projeto. Isso custa caro de três jeitos:

- **Memória:** 3 servidores de banco ociosos, cada um com seus buffers,
  processos de background e manutenção própria. Numa VPS, é o que mais pesa.
- **Operação:** 3 backups para configurar, 3 upgrades de versão, 3 lugares
  para olhar quando algo dá errado.
- **Inconsistência:** cada banco acaba com uma versão, um tuning e uma
  política de backup diferentes.

O objetivo é ter **um** servidor de banco (por motor), e cada projeto usar
**o seu próprio database e o seu próprio usuário** dentro dele. O isolamento
deixa de ser "processos separados" e passa a ser "permissões do banco" — o
mesmo modelo de RDS / Cloud SQL / Atlas.

### Por que isso não dá para fazer "na mão" hoje

O rustploy isola a rede por projeto: cada projeto tem a rede bridge
`rp_net_<id>` e containers de projetos diferentes **não se enxergam** (ver
`AGENTS.md`, seção Segurança). Um banco num projeto `infra` é invisível para a
API do projeto A.

Os contornos manuais são ruins:
- expor a porta do banco no host → o banco fica a uma regra de firewall de
  estar aberto para a internet;
- `docker network connect` na mão → o daemon não sabe disso, e o próximo
  deploy (que recria o container) desfaz a conexão.

Então é preciso uma peça nova no daemon: **uma forma controlada de um serviço
ser alcançável por outros projetos**, sem abrir mão do isolamento por padrão —
e sem criar ambiguidade de nomes, que é o assunto da próxima seção.

## 2. Nomes e aliases: o risco de choque ao integrar redes

### 2.1 Como os nomes funcionam hoje

Dentro de uma rede Docker, cada container é encontrado pelo DNS interno por
**todos** os nomes que tem naquela rede. No rustploy:

| Tipo de serviço | Nome DNS na rede do projeto | Único em… |
|---|---|---|
| Application (Registry/Git/Archive) | nome do container, `rp_<safe_name>` (`docker/containers.rs::replica_live_name`) | **só no projeto** |
| Compose — alias de serviço | a **chave do serviço no YAML**; nos bancos do wizard a chave é `rp_<safe_name>` (`shared/wizard.rs::db_spec`), nos blueprints/compose colado é o que estiver lá (`postgres`, `db`…) | **só no projeto** |
| Compose — nome do container | `rp_<id8>_<safe>-<chave_no_yaml>-1` (`shared::compose_project_name` inclui 8 chars do ID) | **no host inteiro** |

Verificado num Docker local em 2026-09-29, reproduzindo o que o daemon faz
(rede externa do projeto, `-p rp_<id8>_<safe>`, chave `rp_<safe>`): o
container recebe os aliases `rp_abcd1234_meu_banco-rp_meu_banco-1` (nome do
container) e `rp_meu_banco` (chave do YAML), e só esses dois resolvem. O
formato `rp_<nome>-<serviço>-1` descrito em `docs/internal-networking.md`
**não resolve** — é de antes de o `compose_project_name` ganhar o ID; aquele
doc está desatualizado.

A unicidade de nome de serviço é checada **por projeto** (`db/services.rs`,
"já existe um serviço com o nome … neste projeto"). Enquanto cada rede só
contém um projeto, está tudo certo.

### 2.2 O que quebra quando as redes se misturam

Cenário concreto — e é exatamente o cenário da migração:

- Projeto A tem hoje o banco `postgres` → alias `rp_postgres` na `rp_net_A`.
- O servidor central no projeto `infra` também se chama `postgres` → alias
  `rp_postgres`.
- Se as duas coisas ficarem visíveis para a API de A na mesma resolução DNS,
  **`rp_postgres` resolve para os dois**. O Docker devolve um deles (ou os dois,
  em round-robin). A aplicação pode continuar escrevendo no banco **antigo**
  depois da "migração", sem erro nenhum — o pior tipo de falha, a silenciosa.

A revisão 1 deste plano propunha uma rede `rp_shared_<id>` à qual os
**consumidores** se conectavam. Isso tem dois problemas de nome:

1. o choque acima (o container do app fica em duas redes; um nome que existe
   nas duas é ambíguo);
2. os consumidores passam a se enxergar entre si: a `api` de A e a `api` de B
   entram na mesma rede, e `rp_api` passa a ser ambíguo **entre projetos** — a
   API de A pode chamar a API de B achando que é ela mesma.

E um detalhe do Compose piora a coisa: todo serviço Compose ganha, em **toda**
rede declarada no YAML, o nome do serviço como alias. Então injetar uma rede a
mais no YAML (como `inject_project_network` faz com a rede do projeto) espalha
o alias curto para essa rede também — não há como pedir "entre nesta rede sem
o alias do serviço" pelo YAML.

### 2.3 A topologia escolhida: o banco entra na rede dos consumidores

Inverte-se a direção: **o servidor de banco é conectado à rede de cada projeto
autorizado**, e não o contrário.

```
 rp_net_A:  api-A, worker-A, [banco antigo A], servidor-central
 rp_net_B:  api-B,                            servidor-central
 rp_net_C:  api-C,                            servidor-central
 rp_net_infra:                                servidor-central
```

- Consumidores **continuam sem se enxergar** (A nunca entra na rede de B).
- O servidor central é conectado **pelo daemon, depois do `compose up`**, com
  `docker network connect --alias <nome-global> rp_net_A <container>` — não
  pelo YAML. Assim ele entra na rede de A **só** com:
  - o nome do container (`rp_<id8>_<safe>-<serviço>-1`, já único no host), e
  - um alias explícito, **globalmente único**, que é o que vai na connection
    string: `rp-shared-<id8>-<safe>` (ex.: `rp-shared-01j9k2ab-postgres`).
- O alias curto `rp_postgres` do servidor central **não** vaza para a rede de
  A. O banco antigo de A continua sendo `rp_postgres` lá dentro; o central é
  `rp-shared-01j9k2ab-postgres`. Nenhuma ambiguidade, inclusive durante a
  migração, quando os dois coexistem.

Regras que o daemon passa a seguir:

1. **Nenhum nome curto atravessa projeto.** Tudo que atravessa rede de projeto
   usa o prefixo `rp-shared-` + ID. Nomes com `rp-shared-` passam a ser
   **reservados**: criar serviço/serviço Compose cujo nome ou alias comece com
   isso é recusado.
2. **Checagem antes de conectar.** Antes do `network connect`, o daemon
   inspeciona a rede de destino e recusa se o alias já estiver em uso por
   outro container (defesa em profundidade; pela regra 1 não deveria
   acontecer).
3. **Reconectar a cada recriação.** Todo deploy do servidor central recria o
   container; ao fim do `compose up` o daemon reconecta às redes de todos os
   projetos autorizados. O mesmo na recuperação do boot
   (`deploy/recovery.rs`). Revogar acesso = `network disconnect`.
4. **A rede do consumidor precisa existir** — `ensure` antes de conectar
   (projeto sem nenhum deploy ainda pode não ter a rede).

> **Validado num Docker local em 2026-09-29** (mesmo comando do daemon:
> `compose -p … --env-file .env up -d --build --remove-orphans`, com
> `redis:6.2-alpine` no lugar do banco — o que se testa é rede, não o motor):
>
> | Situação | Conexão à rede do projeto A | `rp-shared-…` resolve em A | `rp_postgres` resolve em A |
> |---|---|---|---|
> | logo após `network connect --alias` | presente, aliases = só o explícito | sim | **não** |
> | `compose up` sem mudança | mantida (mesmo container) | sim | não |
> | `docker restart` | mantida | sim | não |
> | `compose up` com env alterada (recria) | **perdida** | **não** | não |
> | `compose up --force-recreate` | **perdida** | **não** | não |
>
> Conclusões: (a) o alias curto do Compose **não** vaza para a rede conectada
> à mão — a premissa da §2.3 vale; (b) toda **recriação** do container desfaz a
> conexão, e recriação acontece em qualquer mudança de env, imagem ou YAML — ou
> seja, em quase todo deploy do servidor central. A regra 3 abaixo **não é
> opcional**: sem ela, o primeiro redeploy do banco derruba todos os projetos
> consumidores, sem erro no deploy do banco (ele sobe saudável; quem quebra são
> os outros). O healthcheck do deploy do servidor central deve incluir "está
> conectado a todas as redes autorizadas".
>
> Não testado: restart do próprio `dockerd`/reboot do host (o container é o
> mesmo, então deve se comportar como o `docker restart`, mas a regra 3 no
> `recovery.rs` cobre de qualquer forma).

### 2.4 Outro choque encontrado, fora do escopo deste plano

Os containers de **Application** se chamam `rp_<safe_name>`, sem ID
(`containers.rs::replica_live_name`). Nome de container Docker é **global no
host**, mas a unicidade de nome de serviço é só **por projeto**. Dois projetos
com um serviço Application chamado `api` disputam o mesmo nome de container —
o segundo deploy falha ou, pior, a troca de container do deploy de um pode
derrubar o do outro. Isso já existe hoje, independente deste plano (os
serviços Compose não têm o problema porque `compose_project_name` inclui o ID).
**Confirmado por leitura de código, e é pior que um conflito:** o deploy acha
o container antigo pelo nome e o remove sem conferir o dono, então o deploy de
B derruba o `api` de A em silêncio. Análise e proteção em
`docs/plano-colisao-nome-container.md`. A topologia da §2.3 não depende disso,
porque nenhum container de Application atravessa rede de projeto.

**Suspeita relacionada — confirmada pelo código e com correção planejada em
`docs/plano-nome-gravado-rede-e-stack.md` §4:** `compose_project_name`
recebe `svc.spec.name`, que muda num rename. Renomear um serviço Compose de
banco mudaria o `-p` do próximo deploy → o Compose criaria uma stack **nova**,
com volume nomeado **novo e vazio** (`<projeto>_pgdata`), enquanto a antiga
continua rodando órfã com o alias antigo. Para um banco, isso parece "perdi os
dados". Confirmar antes de qualquer rename em produção.

## 3. A connection string (`DATABASE_URL`)

### 3.1 O que está errado hoje

A tela do serviço de banco monta duas URLs, ambas em Luau
(`views/scripts/fmt/service_detail.luau`, portado para `webui/fmt.js`):

- **Internal URL** (`internal_url`): `postgresql://rp_<safe>:5432`. Dois
  problemas:
  1. **Sem usuário, sem senha e sem database.**
  2. **O host é um palpite.** `rp_<safe>` **não** é o nome do container
     (esse é `rp_<id8>_<safe>-<chave>-1`, §2.1). Ele só existe na rede se, por
     coincidência, a chave do serviço no YAML for `rp_<safe>` — o que só vale
     para banco criado pelo wizard e **nunca renomeado**. Quebra quando:
     - o banco veio de **blueprint ou compose colado** — a chave é `postgres`,
       `db` etc., e `rp_<safe>` não resolve;
     - o serviço foi **renomeado** — `update_spec` (`db/services.rs`) troca o
       nome mas não reescreve o YAML, então a chave continua `rp_<nome_antigo>`
       e a tela passa a mostrar `rp_<nome_novo>`;
     - a stack tem **mais de um serviço** — a tela não sabe qual deles é o
       banco.

     Além disso `rp_<safe>` tem `_`, que é inválido em hostname pela RFC. O DNS
     do Docker resolve, libpq/mysql/mongo aceitam, mas parsers de URL mais
     estritos (JDBC/`java.net.URI`, alguns validadores) rejeitam o host.
- **URL externa** (`db_connection_url`): para Postgres gera
  `jdbc:postgresql://host:porta/db?user=…&password=…` — formato JDBC, que
  Prisma, SQLAlchemy, node-postgres, sqlx, Django etc. **não** aceitam como
  `DATABASE_URL`. E aponta para o IP externo, não para o nome interno.

Por isso foi preciso escrever a `DATABASE_URL` à mão no último projeto.

### 3.2 O que muda

- A connection string passa a ser montada **no Rust, num lugar só**
  (`crates/shared`, uma função `connection_url(motor, host, porta, database,
  usuário, senha)`, com testes), no formato URI padrão que os drivers aceitam:

  | Motor | Formato |
  |---|---|
  | Postgres | `postgresql://usuario:senha@host:5432/database` |
  | MySQL | `mysql://usuario:senha@host:3306/database` |
  | MariaDB | `mysql://usuario:senha@host:3306/database` (a maioria dos drivers não conhece `mariadb://`) |
  | MongoDB | `mongodb://usuario:senha@host:27017/database?authSource=database` |

  Usuário e senha com percent-encoding (a senha pode ter `@`, `:`, `/`). O
  `authSource` do Mongo é o próprio database, porque é lá que o usuário será
  criado (§4.2) — diferente do root do wizard, que mora no `admin`.
- Ao provisionar um database para um projeto, o daemon grava no projeto
  consumidor uma **env var comum (`EnvVarValue::Plain`)**, **não** um secret.
  Nome padrão `DATABASE_URL` (Mongo: `MONGODB_URI`), editável no formulário.
  Se já existir uma env var com esse nome no projeto, o formulário avisa e
  pede outro nome ou confirmação para sobrescrever (a antiga é guardada para
  rollback, §5.3).
- **O host deixa de ser deduzido do nome do serviço.** Para banco
  compartilhado ele é o alias `rp-shared-<id8>-<safe>`, que o próprio daemon
  atribui (§2.3), então é certo por construção. Para o card "Internal URL" dos
  bancos comuns, o daemon lê o host real: a chave do serviço no YAML
  (`ingress_service` se preenchido; senão, o único serviço da stack; senão,
  aquele cuja imagem bate com o `db_kind`) — ou, com o serviço rodando, os
  `Aliases` do container na rede do projeto. Se não conseguir decidir, mostra
  o nome do container, que sempre resolve, em vez de inventar um.
- O alias do banco compartilhado usa `-` em vez de `_`
  (`rp-shared-<id8>-<safe>`, com os `_` do `safe_name` também trocados por
  `-`: `meu_banco` → `rp-shared-01j9k2ab-meu-banco`). Resolve igual no Docker
  e é hostname válido para qualquer parser.
- A mesma função passa a alimentar o card "Internal URL" das telas atuais
  (GUI e webui), corrigindo-o também para os bancos não compartilhados. O
  cliente recebe a URL pronta do daemon em vez de montar em Luau/JS — deixa
  de existir o risco de as duas cópias (Luau e JS) divergirem.

## 4. Centralização

### 4.1 Servidor compartilhado

Um campo novo no `ServiceSpec`:

```rust
/// Se `Some`, este serviço de banco pode ser conectado às redes de outros
/// projetos autorizados, sob o alias global `rp-shared-<id8>-<safe>`.
pub shared: Option<SharedServerConfig>,
```

Só pode ser marcado como compartilhado um serviço com `db_kind` em
`postgres`, `mysql`, `mariadb` ou `mongodb`. É o `db_kind` que diz ao daemon
qual motor é e onde estão as credenciais de administrador — nas mesmas env
vars que o wizard já grava (`shared/wizard.rs::db_env_vars`):

| Motor | Credencial de admin | Cliente dentro do container |
|---|---|---|
| Postgres | `POSTGRES_USER` (socket local, sem senha) | `psql` |
| MySQL | `root` + `MYSQL_ROOT_PASSWORD` | `mysql` |
| MariaDB | `root` + `MYSQL_ROOT_PASSWORD` | `mariadb` (imagens ≥ 11 não têm mais o binário `mysql`) |
| MongoDB | `MONGO_INITDB_ROOT_USERNAME/PASSWORD`, `authSource=admin` | `mongosh` (imagens ≤ 5 só têm `mongo`) |

Um serviço criado de outro jeito (blueprint, compose colado) não tem
`db_kind`; a opção de compartilhar não aparece nele. Recomendação: criar os
servidores centrais pelo wizard de banco.

Senhas passam para o cliente por variável de ambiente do `docker exec`
(`PGPASSWORD`, `MYSQL_PWD`) ou stdin, nunca como argumento de linha de
comando (que aparece no `ps` do host).

### 4.2 Databases gerenciados

Na tela do servidor compartilhado, aba **"Databases"**: lista + botão **"Novo
database"**. O formulário pede o nome do database, o projeto consumidor e o
nome da env var (padrão da §3.2). Ao confirmar, o daemon gera uma senha e roda,
via `docker exec` no container do servidor:

**Postgres**
```sql
CREATE ROLE projeto_a LOGIN PASSWORD '<gerada>';
CREATE DATABASE projeto_a OWNER projeto_a;
REVOKE ALL ON DATABASE projeto_a FROM PUBLIC;
```

**MySQL / MariaDB**
```sql
CREATE DATABASE `projeto_a`;
CREATE USER 'projeto_a'@'%' IDENTIFIED BY '<gerada>';
GRANT ALL PRIVILEGES ON `projeto_a`.* TO 'projeto_a'@'%';
```
(`'%'` porque o cliente chega de outro container, com IP variável; o que
restringe quem chega é a rede, §2.3.)

**MongoDB**
```js
db.getSiblingDB("projeto_a").createUser({
  user: "projeto_a", pwd: "<gerada>",
  roles: [{ role: "readWrite", db: "projeto_a" }]
})
```
(no Mongo o database só passa a existir na primeira escrita; o usuário é
criado dentro dele, daí o `authSource=projeto_a`.)

Depois: conecta o servidor à rede do projeto consumidor (§2.3), grava a env
var Plain (§3.2) e registra o acesso no banco do rustploy. No próximo deploy
dos serviços do projeto a `DATABASE_URL` já resolve.

Nomes de database/usuário são validados como `[a-z0-9_]`, até 32 chars
(limite do usuário no MySQL), antes de irem para o SQL — nada de concatenar
texto livre em comando.

### 4.3 Limites por projeto (vizinho barulhento)

Oferecidos no mesmo formulário, opcionais:

| Motor | Limite de conexões | Timeout de query |
|---|---|---|
| Postgres | `ALTER ROLE … CONNECTION LIMIT n` | `ALTER ROLE … SET statement_timeout` |
| MySQL | `ALTER USER … WITH MAX_USER_CONNECTIONS n` | `max_execution_time` (só `SELECT`) |
| MariaDB | `ALTER USER … WITH MAX_USER_CONNECTIONS n` | `max_statement_time` |
| MongoDB | não tem por usuário | não tem por usuário (só `maxTimeMS` pela aplicação) |

Para o Mongo, a tela diz com todas as letras que não há como limitar.

### 4.4 Fora de escopo (de propósito)

- **Redis e brokers.** Não têm "database com permissão" de verdade; um Redis
  compartilhado é outro problema.
- **Migrar entre motores** (Postgres → MySQL etc.). A migração é sempre do
  mesmo motor para o mesmo motor. MySQL ↔ MariaDB também **não** é permitido
  na primeira versão: os dumps são quase compatíveis, e o "quase" é onde mora
  o problema (collations, tipos JSON, sintaxe de usuários).
- **Não é um DBaaS genérico.** O servidor continua sendo um serviço Compose
  normal, com volume, versão e config sob controle do usuário.

## 5. Migração dos dados antigos

Reaproveita a infra de **Jobs** (compose efêmero, log ao vivo, exit code
decide sucesso/falha, cancelamento). O que é novo é o **assistente** que monta
o job certo e cuida dos passos antes e depois.

### 5.0 Como o dump é feito: binário nativo de cada banco, não biblioteca

O dump e o restore são feitos pelas **ferramentas oficiais de cada motor**,
rodando dentro de um container do Job — o daemon não lê nem escreve dado
nenhum, e não ganha driver de Postgres/MySQL/Mongo como dependência.

**Nada é instalado no sistema operacional do servidor.** Tudo o que este plano
executa roda em container: o provisionamento (§4.2) e a verificação (§5.2,
passo 4) via `docker exec` no container do próprio servidor de banco, com o
cliente que já vem na imagem dele; o dump/restore num container efêmero do
Job. O host só precisa do Docker + Compose que o rustploy já exige.

| Motor | Dump | Restore | Vem de |
|---|---|---|---|
| Postgres | `pg_dump` | `pg_restore` | imagem oficial `postgres` |
| MySQL | `mysqldump` | `mysql` | imagem oficial `mysql` |
| MariaDB | `mariadb-dump` | `mariadb` | imagem oficial `mariadb` (pacote `mariadb-server`) |
| MongoDB | `mongodump` | `mongorestore` | imagem oficial `mongo` (pacote `mongodb-org-tools`, conferido no Dockerfile de `mongo:8.0`) |

Para o MariaDB usam-se os nomes `mariadb-dump`/`mariadb`, que existem desde a
10.5; os nomes `mysqldump`/`mysql` são só compatibilidade e já não estão em
todas as imagens recentes.

**Por que não uma biblioteca (crate Rust):**

- **Fidelidade.** Um dump correto não é "copiar as linhas": é copiar
  sequences, índices, constraints, views, triggers, functions, extensions,
  tipos próprios (Postgres), procedures/events e collations (MySQL), tipos
  BSON e opções de coleção (Mongo), na ordem em que o restore aceita. As
  ferramentas oficiais fazem isso e são mantidas junto com o motor; não existe
  crate que faça o equivalente, e reescrever seria criar a parte mais arriscada
  do projeto do zero.
- **Compatibilidade de versão.** Cada ferramenta sabe ler as versões
  anteriores do próprio motor. Isso vem de graça ao usar a ferramenta da
  versão certa (abaixo).
- **Os dados não passam pelo daemon.** O pipe `dump | restore` roda inteiro
  dentro do container do Job, de rede Docker para rede Docker. Um banco de
  vários GB não ocupa memória nem CPU do `rustployd`, e uma falha no dump não
  derruba o daemon.
- **É o que o rustploy já faz.** Jobs já sobem containers efêmeros com
  imagem arbitrária e decidem pelo exit code; a migração é "só" um Job com o
  comando certo.

O que o daemon faz é **orquestrar**: montar o comando conforme o motor,
passar as credenciais, subir o Job, ler o exit code e o log, e rodar os passos
antes e depois (§5.2).

**Qual imagem:** a **mesma imagem (e tag) do servidor de destino**. O destino é
≥ a origem (pré-checagem), e cada ferramenta aceita servidor de versão igual ou
**anterior** à dela — o `pg_dump`, por exemplo, recusa servidor mais novo que
ele ("server version mismatch"). Usar a versão do destino satisfaz as duas
pontas, e a imagem já está baixada no servidor.

**Credenciais: nunca na linha de comando.** Os argumentos de um processo
dentro de container aparecem no `ps` **do host**. Então:

| Motor | Como a senha chega |
|---|---|
| Postgres | env `PGPASSWORD`, uma por ponta (`env PGPASSWORD=… pg_dump …` dentro do script do job) |
| MySQL / MariaDB | arquivo `--defaults-extra-file` por ponta (`[client] user/password/host`), gerado no container |
| MongoDB | `--config` (arquivo YAML com `password:`, suportado pelos database tools) + `--username` / `--authenticationDatabase`; a URI vai **sem** senha |

Os arquivos de credencial são escritos num `tmpfs` do container do Job e somem
com ele. As senhas vêm das env vars do serviço de origem e do database
gerenciado — o daemon as passa como env vars do Job, como qualquer Job já
recebe.

**Rede do job:** ele roda com o serviço de origem como serviço gatilho, logo
entra na `rp_net_A`. Pela §2.3 o servidor central também já está na
`rp_net_A` (o database foi provisionado antes). O job enxerga os dois — o
antigo como `rp_postgres`, o novo como `rp-shared-<id8>-postgres` — sem
ambiguidade.

### 5.1 O assistente "Migrar para banco compartilhado"

Botão na tela do serviço de banco antigo (precisa ter `db_kind`). Pede:

- **origem:** este serviço (já preenchido) + nome do database de origem;
- **destino:** um database gerenciado de um servidor compartilhado **do mesmo
  motor** (ou "criar agora", que dispara a §4.2);
- **env var a trocar:** qual env var dos serviços do projeto aponta para o
  banco (padrão `DATABASE_URL`).

### 5.2 Os passos, na ordem

Cada passo aparece na tela com o seu estado (pendente / rodando / ok /
falhou), como a máquina de estados do deploy.

1. **Pré-checagem** — as duas pontas respondem? Mesmo motor? Versão de
   destino ≥ origem? Espaço em disco no volume do destino para o tamanho do
   database de origem? O destino está vazio (não sobrescrever um database em
   uso)?
2. **Parar escritas** — para os serviços do projeto que usam o banco antigo
   (não o banco em si).
3. **Dump + restore**, com as ferramentas da §5.0, em pipe — sem arquivo
   intermediário, então não precisa de espaço em disco para o dump, só para os
   dados no destino. `set -o pipefail` no script, senão uma falha do dump no
   meio do pipe passa despercebida (o exit code seria só o do restore):

   **Postgres**
   ```sh
   env PGPASSWORD="$ORIGEM_PASS" pg_dump --format=custom --no-owner --no-acl \
       -h "$ORIGEM_HOST" -U "$ORIGEM_USER" "$DB_ORIGEM" \
     | env PGPASSWORD="$DESTINO_PASS" pg_restore --no-owner --role=projeto_a \
       --exit-on-error -h "$DESTINO_HOST" -U projeto_a --dbname=projeto_a
   ```
   (`env VAR=… cmd` dentro do script do job não aparece como argumento do
   processo `pg_dump`; o valor fica no ambiente dele, que o `ps` não mostra.)

   **MySQL**
   ```sh
   mysqldump --defaults-extra-file=/run/creds/origem.cnf \
             --single-transaction --routines --triggers --events \
             --no-tablespaces --set-gtid-purged=OFF "$DB_ORIGEM" \
     | sed -E 's/DEFINER=`[^`]+`@`[^`]+`/DEFINER=CURRENT_USER/g' \
     | mysql --defaults-extra-file=/run/creds/destino.cnf projeto_a
   ```

   **MariaDB** — o mesmo, com `mariadb-dump` / `mariadb` e **sem**
   `--set-gtid-purged` (só o `mysqldump` do MySQL aceita).

   O `sed` troca o dono de views/triggers/procedures: sem isso o restore
   falha, porque o usuário antigo não existe no servidor novo.

   **MongoDB**
   ```sh
   mongodump --config=/run/creds/origem.yaml --host="$ORIGEM_HOST" \
             --username="$ORIGEM_USER" --authenticationDatabase=admin \
             --db="$DB_ORIGEM" --archive \
     | mongorestore --config=/run/creds/destino.yaml --host="$DESTINO_HOST" \
             --username=projeto_a --authenticationDatabase=projeto_a \
             --archive --nsFrom="$DB_ORIGEM.*" --nsTo="projeto_a.*"
   ```
   A origem usa o root do wizard (`authSource=admin`); o destino, o usuário
   novo (`readWrite` no database dele — suficiente para criar coleções e
   índices). Usuários e roles da origem não são copiados, de propósito.

4. **Verificação** — conta registros por tabela/coleção nas duas pontas e
   compara. Diferença = falha, e a migração para aqui.
   - Postgres: `count(*)` por tabela; confere também as sequences.
   - MySQL/MariaDB: `COUNT(*)` por tabela. **Não** usar
     `information_schema.TABLES.TABLE_ROWS`: no InnoDB é uma estimativa.
   - MongoDB: `countDocuments({})` por coleção + contagem de índices.
5. **Trocar a conexão** — a env var escolhida no projeto passa a ter a URL
   nova (Plain, §3.2). **O valor antigo é guardado**, não apagado.
6. **Subir os serviços** — redeploy dos serviços parados no passo 2, com o
   healthcheck normal decidindo se deu certo.
7. **Período de observação** — o banco antigo **continua existindo, parado**.
   A tela mostra "Migração concluída — banco antigo mantido para rollback" com
   dois botões: **Reverter** e **Descartar banco antigo**.

### 5.3 Rollback

Se algo der errado **até o passo 6**, o assistente desfaz sozinho: restaura a
env var antiga e sobe os serviços de volta no banco antigo, que nunca foi
alterado (o dump só *lê* a origem).

Depois do passo 6, o botão **Reverter** faz o mesmo manualmente — com o aviso
claro de que **o que foi escrito no banco novo depois da migração se perde**.

O banco antigo só é removido quando o usuário clica em "Descartar". Nunca
automaticamente.

### 5.4 Com parada ou sem parada?

Primeira versão: **só com parada** (os passos acima). A aplicação fica fora do
ar pelo tempo do dump + restore — minutos para bancos de alguns GB — e é
impossível perder dado.

Sem parada exigiria mecanismos diferentes por motor (replicação lógica no
Postgres, binlog no MySQL, change streams / replica set no Mongo), cada um com
pré-requisitos que pedem restart do banco antigo. Não compensa para 3
projetos.

## 6. Como eu faria a migração dos seus 3 projetos

1. Criar o projeto `infra` com um servidor **por motor em uso** (se os 3 forem
   Postgres, um só), pelo wizard de banco, com a **maior versão** entre os
   bancos atuais, ou mais nova. Marcar como compartilhado.
2. Configurar backup desse servidor **antes** de migrar qualquer coisa — ele
   passa a ser o ponto único de falha dos projetos.
3. Migrar **o projeto menos crítico primeiro**, com parada, num horário calmo.
4. Deixar rodando alguns dias. Só então migrar o segundo, e depois o terceiro.
5. Descartar os bancos antigos só depois de todos estáveis.

## 7. O custo da centralização

- **Ponto único de falha:** se o servidor central cai, os projetos caem
  juntos.
- **Vizinho barulhento:** uma query pesada de um projeto deixa os outros
  lentos. Mitigável pelos limites da §4.3 (menos no Mongo).
- **Upgrade de versão vira conjunto:** não dá para subir só o banco de um
  projeto.
- **O servidor central fica em várias redes** (§2.3): ele enxerga os
  consumidores, embora os consumidores não se enxerguem. Um banco comprometido
  alcança os apps de todos os projetos — o mesmo que já seria verdade de
  qualquer banco compartilhado.

Para 3 projetos numa VPS, o ganho de memória e de simplicidade de backup
normalmente compensa. Se algum projeto for muito mais crítico, pode ficar de
fora.

## 8. Onde cada peça moraria

| Peça | Onde |
|---|---|
| `ServiceSpec.shared`, `SharedAccess`, `ManagedDatabase`, `connection_url()` | `crates/shared/src/models.rs` (+ testes da URL) |
| Tabelas novas (acessos, databases gerenciados, migrações) | `crates/daemon/src/db/` |
| Alias global, `network connect/disconnect`, checagem de alias em uso | `crates/daemon/src/docker/networks.rs` |
| Reconectar após `compose up` e no boot | `deploy/executor.rs`, `deploy/recovery.rs` |
| Nomes `rp-shared-*` reservados | `db/services.rs` (validação de nome) |
| Provisionar database/usuário, por motor (`docker exec`) | módulo novo no daemon, um `impl` por motor |
| Migração (passos + Job de dump/restore por motor) | reaproveita `crates/daemon/src/jobs/` |
| Telas (aba Databases, assistente de migração, Internal URL vinda do daemon) | **GUI iced e webui** — as duas |

## 9. Fases sugeridas

1. **`connection_url()` no Rust + card "Internal URL" corrigido** (§3). Pequena,
   independente, e já resolve a `DATABASE_URL` escrita à mão.
2. **Servidor compartilhado + conexão às redes dos consumidores com alias
   global** (§2.3, §4.1). Resolve o problema de fundo: com isso a migração já
   pode ser feita à mão, com um Job.
3. **Databases gerenciados** (§4.2, §4.3), Postgres primeiro, depois MySQL/
   MariaDB e MongoDB — a estrutura é a mesma, muda o `impl` do motor.
4. **Assistente de migração** (§5), com parada, na mesma ordem de motores.
5. (Separado) corrigir o choque de nome de container de Application entre
   projetos (§2.4).

Se o prazo da tarefa em produção for curto, as **Fases 1 e 2** já destravam a
migração — o resto é conforto e segurança de operação.
