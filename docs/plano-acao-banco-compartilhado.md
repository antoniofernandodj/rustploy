# Plano de ação: implementar o banco compartilhado (Postgres) e migrar rdo-itemize e gestão

> **Status:** aprovado e em execução (2026-09-30). Decisões: só Postgres primeiro, depois
> MySQL/MariaDB/Mongo; as 4 fases seguidas; a máquina local é só de teste (sem
> cuidado especial com o daemon instalado).
>
> **Progresso:** ver §8 ao final.
> Complementa `docs/plano-banco-compartilhado.md` (o *desenho*); este é o *como
> executar e testar*. Escopo desta rodada: **só Postgres**.

## 1. O que o zip `rustploy-infra.zip` realmente é

Não é uma infra rodando: é um **export IaC** (`rustploy.yml` + `rustploy.vars.toml`)
com 4 projetos. Os que interessam:

| Projeto | Banco hoje | Como a app o acha |
|---|---|---|
| `rdo-itemize` | serviço `rdo-banco` (Compose, `postgres:18`, chave YAML `banco`, `db: postgres`, volume `dados_banco`) | `URL_BANCO_DADOS` (env do projeto) |
| `gestão` | serviço `db` (Compose, `postgres:18`, chave `rp_db`, `db: postgres`, volume `pgdata`) | `DATABASE_URL` (env do serviço `api`) |
| `Standimob` | Postgres **dentro** do stack Supabase | fora de escopo (não é `db_kind`, é stack próprio) |
| `Itemize Institucional` | nenhum | fora |

Os dois bancos são `postgres:18`, então o servidor central também é `postgres:18`
(destino ≥ origem, §5.2 do plano de desenho). Os dados **reais** só existem na VPS;
o zip traz a *configuração*. Por isso os testes usam **dados sintéticos** nos
bancos locais e a migração real é um passo de produção à parte (§6).

## 2. Regra de segurança dos testes (importante)

Esta máquina já tem um `rustployd` **de verdade** rodando (`/usr/bin/rustployd`,
pid 3542, API em 127.0.0.1:9797) e containers alheios (`nfe_watcher-*`, rede
`rp_01m11wf7_supabase_default`). Um daemon de desenvolvimento no mesmo socket
Docker compartilharia tudo isso. Então:

- daemon de teste com **config própria** (portas, `db_path`, `master_key` em
  diretório temporário; ingress em portas altas; `acme.enabled = false`);
- **limpeza automática desligada** (já vem desligada por padrão) e o teste
  nunca roda `prune` do manifesto;
- importar o `rustploy.yml` **só dentro do daemon de teste**, renomeando os
  projetos com prefixo `t-` e trocando `host_port` para faixa livre, para não
  colidir com portas nem nomes de containers do que já roda;
- ao final, remover só o que tem o prefixo de teste.
- Não tocar o pid 3542 nem `~/.config/rustploy`.

## 3. Ordem de implementação (as 4 fases do doc de desenho, Postgres apenas)

Cada fase termina com `cargo test` dos pacotes afetados **e** execução real
contra o daemon de teste (regra do projeto: teste verde não basta p/ UI).

**Fase 1 — `connection_url()` + card "Internal URL"** (`crates/shared`)
- Função única (Postgres agora; assinatura já aceita os outros motores),
  percent-encoding de usuário/senha, testes de unidade (senha com `@ : /`).
- Daemon passa a entregar a URL pronta; GUI (Luau) e webui (`fmt.js`) deixam de
  montar a sua. Host = chave real do serviço no YAML / alias / nome do container.
- Entrega imediata: acaba a `DATABASE_URL` escrita à mão.

**Fase 2 — servidor compartilhado + rede com alias global**
- `ServiceSpec.shared: Option<SharedServerConfig>` (só com `db_kind` Postgres
  nesta rodada); migração de tabela para os acessos.
- `docker/networks.rs`: ativar/usar `connect` com `--alias rp-shared-<id8>-<safe>`
  (hoje só existem `_connect_container` sem alias), checagem de alias em uso,
  `disconnect`.
- Reconexão após todo `compose up` do servidor (`deploy/executor.rs`) e no boot
  (`deploy/recovery.rs`) — **obrigatório**, o doc de desenho validou que a
  recriação derruba a conexão.
- Nomes `rp-shared-*` reservados (`db/services.rs`).
- Teste: servidor + 2 projetos-consumidor sintéticos; provar que o alias
  resolve nos dois, que `rp_postgres` do banco antigo **não** vaza, que um
  redeploy do servidor reconecta, e que A e B **não** se enxergam.

**Fase 3 — databases gerenciados (Postgres)**
- Módulo novo no daemon, um `impl` por motor (só Postgres agora, trait pronta
  p/ os demais): `CREATE ROLE/DATABASE/REVOKE` via `docker exec` (senha por
  env, nome validado `[a-z0-9_]{1,32}`), limites (`CONNECTION LIMIT`,
  `statement_timeout`), grava env var **Plain** no projeto consumidor.
- API + aba "Databases" na **GUI e na webui**.

**Fase 4 — assistente de migração (com parada)**
- Reaproveita `jobs/` (e o plano `plano-jobs-na-fila-de-deploy.md`, já no
  HEAD: jobs e deploys dividem uma vaga): job com a imagem `postgres:18`,
  `set -o pipefail`, `pg_dump | pg_restore`, credenciais por `PGPASSWORD`.
- Passos: pré-checagem → parar escritas → dump/restore → verificação
  (`count(*)` por tabela + sequences) → trocar env → subir → observação, com
  rollback automático até o passo 6. Banco antigo nunca removido sozinho.
- Teste de ponta a ponta nos dois projetos sintéticos (tabelas, sequences,
  FK, view, extensão, >1 schema).

## 4. Testes contra a infra do zip

1. Subir daemon de teste isolado (§2).
2. Importar o manifesto (`ManifestImport`) com o prefixo `t-`, **só** os projetos
   `rdo-itemize` e `gestão` (as apps com `git` não precisam buildar: usa-se só
   os serviços de banco + um consumidor mínimo, p.ex. um container `postgres`
   cliente, que exercita a `DATABASE_URL`).
3. Popular os dois bancos antigos com dados sintéticos.
4. Criar `infra`/servidor central `postgres:18` (wizard), marcar compartilhado,
   provisionar um database por projeto, migrar, verificar contagens, trocar a
   env var e subir o consumidor contra o banco novo; testar rollback e
   "Reverter" em pelo menos um dos dois.
5. Redeploy do servidor central → consumidores continuam conectando.

## 5. Decisões que preciso de você

1. **Escopo:** só Postgres agora (recomendado — é o que você pediu e o que
   existe nos dois projetos), ou já construir MySQL/MariaDB/Mongo?
2. **Fases:** implementar as 4 seguidas, ou parar após a Fase 2 (que já
   destrava migrar à mão com um Job) e reavaliar?
3. **Doc de desenho:** ele segue "proposta, não implementado"; atualizo o
   status conforme cada fase entra?

## 6. Produção (não será feito por mim sem você pedir)

A migração dos dados **reais** (VPS) é operação sua: criar `infra`, configurar
backup do servidor central **antes**, migrar o menos crítico primeiro, observar
alguns dias, só então descartar os bancos antigos. Este trabalho entrega a
ferramenta e prova que funciona no teste; não conecto na VPS nem no daemon
pid 3542.

## 7. Commits

Um commit por fase, na `main`, com `make index` (índice de código) regenerado
quando símbolos mudarem. Há mudanças não commitadas suas no working tree
(`service.gv`, docs deletados/`docs/arquivo/`); não vou misturá-las nos meus
commits (adiciono só os arquivos de cada fase).

## 8. Progresso

- **Fase 1 — concluída (2026-09-30).** `shared::connection` (`connection_url`,
  `credentials`, `compose_host`, 5 testes), comando `ServiceConnectionInfo`
  (daemon), GUI e webui consomem a URL pronta. Verificado num daemon de teste
  com o `rustploy.yml` do zip importado: `rdo-banco` →
  `postgresql://…@banco:5432/…` (o host é a chave do YAML, **não** `rp_banco`,
  confirmando o palpite errado que o doc de desenho descreve).
  O formato da **URL externa** (JDBC no Postgres) não foi tocado.

- **Fase 2 — concluída (2026-09-30).** `ServiceSpec.shared`, tabela
  `shared_access`, `deploy/shared_net.rs` (conecta o servidor às redes
  autorizadas só com o alias `rp-shared-<id8>-<safe>`, derivado da stack
  gravada — estável no rename), comandos `SharedAccess{List,Grant,Revoke}`,
  prefixo `rp-shared-` reservado e `shared` só em banco Compose (validado em
  `db::services::create/update_spec`), campo `shared` no manifesto IaC.
  Reconexão: após cada `compose up` do servidor e a cada volta do reconcile (30 s,
  também no boot).
  **Verificado de verdade** (daemon de teste, manifesto do zip importado, servidor
  `postgres:18` no projeto `infra`, acesso a `rdo-itemize` e `gestão`): o alias
  resolve nas duas redes e o psql conecta por ele; `rp_pg_central` (alias
  curto) **não** vaza; recriar o container reconecta na hora; desconectar à mão
  é curado pelo reconcile em ≤35 s. Achado no teste: `attach_network_alias`
  presume o container já na rede (falhava com "not connected"); criei
  `connect_with_alias` próprio.
  **UI fica para a Fase 3**: o liga/desliga "compartilhado" e a concessão de
  acesso entram junto com a aba "Databases" (conceder é efeito de criar um
  database para um projeto). Até lá, só pela API.
