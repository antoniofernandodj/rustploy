# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

Ordenada por valor (o mais valioso primeiro, avaliação de 2026-10-19). Ao
acrescentar um item, encaixe-o na posição certa em vez de colar no fim.

- [x] **Publicar o `rustploy-shared` 0.16.0 e tirar o `path` do daemon** (decisão de 2026-11-03: até lá `cargo install
  rustploy` entrega a 0.1.28 e o `rustploy-daemon` não compila sozinho). Passos em `docs/release.md` (v0.1.35).
- [ ] **Chave S3 própria por app** no Storage (hoje a app usaria a chave root) e rotacionar a senha do `db` do Chiquitos
  (foi exibida a um agente antes da Fase A). Ver `docs/plano-credenciais-cifradas-e-storage.md`.
- [ ] **Template `minio` do catálogo** aponta para `minio/minio` (não puxável): trocar a imagem no blueprint.
- [x] **Release com o `rustploy-podman-ufw`** (commitado no daemon, ainda sem release): só então fazer o deploy das docs (a
  seção "Podman e ufw" e a receita de bucket com `curl` já estão commitadas, mas o site ainda tem a receita errada com `aws-cli`).
  (O isolamento entre dois projetos já tem teste de laboratório; falta só rodar num servidor real com dois projetos.)
- [ ] **Aposentar o `db_kind`: renomear para `product` (planejado em 2026-11-04, nada feito).** O campo nasceu quando só havia
  banco; hoje guarda também broker e storage (`postgres`, `mongodb`, `mariadb`, `mysql`, `redis`, `kafka`, `rabbitmq`, `nats`,
  `rustfs`, `minio`) e o nome mente. Meta: **zero ocorrências de `db_kind`** no código, nos YAMLs de produção, nos docs e no índice.
  - **Nome proposto: `product`** (`ServiceSpec.product: Option<String>`) — é o *produto de software gerenciado* que roda no
    serviço, vale para qualquer um da lista acima e para um futuro (`clickhouse`, `meilisearch`…); `None` = serviço comum
    (app/compose). Descartados: `engine` (colide com `platform::engine` Docker/Podman e `databases::Engine`), `kind` (genérico
    demais, já usado em `git_provider.kind`/`StorageKind`), `service_type` (confunde com `ServiceSource`).
    **Confirmar o nome com o dono antes de começar.**
  - **Escopo (≈ 90 ocorrências, 28 arquivos):** `rustploy-shared` (`domain/service.rs`, `ingress.rs`, `deployment.rs`: campo + doc comment,
    que ainda diz "banco ou broker"); `rustploy-daemon` (22 arquivos: `features/wizard.rs`, `databases/{managed_view,shared_db,migration}.rs`,
    `deploy/{shared_net,autostart}.rs`, `iac/{manifest,service_import,manifest_import,service_bundle}.rs`, `ingress/router.rs`,
    `store/services.rs`, `api/handlers/{services,databases}/*`, `api/http/dashboard.rs`, `platform/connection.rs`, `webui/app.js`,
    `webui/screens/service_detail.js`, `api/web_ui/headless_tests.rs`); `rustploy-gui` (`scripts/format/service_detail.luau`,
    `scripts/handlers/services.luau`, README). Renomear também **parâmetros, variáveis e funções** que carregam o nome
    (`internal_scheme(db_kind)`, `database_connection_url(db_kind…)`, `ctx.service_database_kind`, `Engine::from_kind`…), não só o campo;
    helpers como `Engine::from_kind` ficam se o nome ainda fizer sentido para o recorte "só bancos". A chave `db:` do manifesto
    de IaC (`ServiceManifest.db`, `manifest.rs:209`) vira `product:` junto — é o mesmo dado.
  - **Dado em produção — ponto que pode quebrar (`serde(default)` engole a chave velha em silêncio):** o `ServiceSpec` é
    gravado como está em `projects/<id>/project.yml` (store/infra, `SCHEMA = 1`). Se o campo mudar de nome sem migração, o parse
    **não falha**: `db_kind:` vira campo desconhecido, `product` fica `None` e todo banco/broker/storage passa a ser tratado como
    serviço comum (sem aba Connection, sem URL interna, sem rede compartilhada, sem autostart de dependência). Por isso:
    1. **Migração de dados, não alias.** Subir `SCHEMA` para 2 e fazer `store::infra::migrate` reescrever `db_kind:` → `product:` em todo
       `project.yml` na primeira carga (com backup do diretório antes, como o `backup_sqlite` já faz), gravando só depois de reler
       com sucesso. Sem `#[serde(alias = "db_kind")]` permanente — ele manteria a referência viva. Se for preciso um alias para
       fazer a leitura de transição, entra **no mesmo commit** da migração e sai na release seguinte.
    2. **Espelho em git (`state_git`)**: o repositório remoto também tem `project.yml` com `db_kind`. A migração roda no diretório de
       infra e o próximo sync/commit empurra o arquivo novo; conferir que o daemon novo **não** lê o repositório velho de volta
       por cima (divergência) — testar em modo `commit` antes de `sync`.
    3. **Editar o YAML de produção à mão só como plano B.** Cobrir os dois servidores (chiquitos e itemize). Comando de
       conferência: `grep -rn db_kind <dir de infra>` tem que voltar vazio.
    4. **Manifestos exportados** (`rustploy.yml` de "Exportar tudo") feitos antes: o importador deve recusar `db:` com mensagem clara
       apontando para `product:` (ou aceitar com aviso por uma release), nunca ignorar em silêncio.
  - **Compatibilidade de versões:** JSON da API troca de nome junto — GUI e webui velhas contra daemon novo perdem as abas de
    banco/broker/storage (sem erro). Ordem: publicar `rustploy-shared` (**0.17.0**, depois da 0.16.0 pendente acima) → daemon (com
    migração) → GUI/webui na **mesma release**; avisar que a GUI antiga precisa atualizar. A webui vem embutida no daemon, então já
    acompanha.
  - **Testes que provam:** (a) teste de migração com um `project.yml` real da produção como fixture (banco, broker, storage e um
    serviço sem `db_kind`), antes/depois; (b) o teste de round-trip do `infra` com `product` preenchido; (c) `headless_tests.rs` e os
    testes Luau (`glacier test`) já existentes, com o nome novo; (d) smoke em servidor de teste: abrir um postgres, um kafka e um
    rustfs e ver a aba Connection.
  - **Fecho:** `grep -rIn "db_kind" . --exclude-dir=target --exclude-dir=.git` vazio (inclui `docs/`, `docs/indice/*`, `README`s,
    `AGENTS.md`); rodar `make index`; atualizar a memória do projeto. Docs históricos em `docs/arquivo/` que citam `db_kind` ganham
    uma nota "renomeado para `product` em 2026-11" em vez de reescrita, mas a referência literal também some (trocar o termo).
  - **Fica de fora (decisão pendente):** transformar `product` num enum tipado (`ServiceProduct { Postgres, Kafka, MinIo… }`) em vez
    de `String`. Resolveria o "banco é o que sobra" de `dashboard::classify`, mas muda o protocolo e o YAML; deixar para depois do
    rename, que já é grande por si.
- [ ] **Geração e rotação de segredos** (`SecretGenerate`, `SecretRotate`, par VAPID/Ed25519, botões "Gerar"/"Rotacionar", `generate:` no manifesto,
  job-modelo por motor). Só projetado, em `docs/plano-geracao-e-rotacao-de-segredos.md`; faltam as respostas do dono (seções 2 e 3 do plano).
- [ ] **Webhook: o daemon deve aceitar os dois formatos de corpo, `application/json` e `application/x-www-form-urlencoded`** (`payload=<json>`,
  com percent-decoding; escolher pelo `Content-Type`). O padrão da UI do GitHub é `form`, e hoje nele o `watch_paths` e o filtro de branch são
  ignorados em silêncio (deploya tudo); só funciona com `content_type: json`. Ajustar `extract_push_branch`/`extract_changed_files` em
  `api/http/public_routes.rs` (já existe um parser de form no arquivo), com testes dos dois formatos, e atualizar o guia de webhooks (pt/en).
  Considerar também ignorar o `ping` do GitHub, que hoje dispara um deploy.
- [ ] **Sem registro de deploy da api e do worker após a atualização para 0.1.33** (Chiquitos, 2026-11-05): os containers rodam (api
  `rp_api:01M6JF8R`), mas `DeployHistory` da api volta vazio e os deploys `dep_01M6JF8R…` (api) e `dep_01M6JDZ7…` (worker) não aparecem em
  `list_deploys`; `live_container_id` é `null` nos dois (o do scheduler está certo). `updated_at` de ambos = 01:27:36, ~5 min depois do daemon subir.
  Investigar o que o início da 0.1.33 (ou a recuperação) faz com serviços Git; antes do próximo deploy deles, conferir se o deploy cria o registro.
- [x] **Migrations embutidas no `chickie-setup`** (`include_dir!` + `build.rs` com `rerun-if-changed=migrations`) em vez de ler `.sql` do disco; tira o `COPY migrations` da imagem.
- [x] **Redeploy do worker e do scheduler** com o código novo (Fase G: a api já foi verificada) e **ajustar a receita de bucket nas docs** (ainda ensina `curl`).
- [ ] **RabbitMQ: "no heartbeat received from server for too long"** no log da api (2 erros 2 min após o boot, 2026-11-04 04:23); investigar se é pré-existente.
- [x] **Assinar a release `v0.1.38`** e descartar rascunhos de `v0.1.36`/`v0.1.37`, se existirem (as tags não puderam ser apagadas).
- [ ] **Chickie**: as branches `deploy/api`, `deploy/worker` e `deploy/scheduler` (no repo do chickie) têm o ajuste de URLs por partes,
  o cache `.sqlx` regenerado (a `main-api` não compila offline sem ele), o job de migrations e as filas duráveis do RabbitMQ 4.
  Levar para a `main` quando o dono decidir. Preencher no servidor: SMTP, Asaas, VAPID, `JWT_SECRET`, `OWNER_EMAIL`, `FRONTEND_BASE_URL`.
- [x] **PRIORIDADE ZERO — concluída em 2026-10-19 (falta só conferir o `.exe` do Windows): reduzir o Rust da GUI ao mínimo.** Análise de 2026-10-19. O
  Rust do `rustploy-gui` já é só a casca (~560 linhas de código real fora testes; o Luau/`.gvb`/`.gss`
  somam ~16.300). Para chegar a um `main.rs` de ~5 linhas (`glacier_ui::run("views/")`), o que sobrou
  precisa migrar para o glacier-ui. Do mais ao menos viável:
  1. ~~`src/manifest_zip.rs`~~ — **feito em 2026-10-19**: virou `views/scripts/manifest_zip.luau` sobre
     `zip_write`/`zip_read` do glacier-ui 0.123; testes Luau em `tests/` rodam com `glacier test` (CLI 0.7.1).
  2. ~~`src/desktop.rs`~~ e 3. ~~`src/embedded.rs` + `src/assets.rs`~~ — **feitos em 2026-10-19** (glacier-ui 0.125.1):
     `GlacierDaemon::install_desktop`, `embed_dirs!` e `locate_assets_and_chdir`. `embedded.rs` ficou só
     com a lista de pastas (~14 linhas); o `main.rs` tem 22. Sobra o `build.rs` (item 4).
  4. ~~`build.rs`~~ — **feito em 2026-10-19**: a lógica foi para o crate novo `glacier-build 0.1`
     (`stage_images`, `desktop_on_cargo_install`, `windows_resource`) e o `build.rs` ficou com 18 linhas.
     Não validado: o cross-build do `.exe` do Windows (a lógica de versão foi copiada igual).
  Observação: migrar troca o Rust do Rustploy por Rust do glacier-ui, o código continua existindo.
  Cada mudança no glacier-ui segue a regra do crate (nunca `path`/`[patch]`: publicar, depois subir
  a versão no `rustploy-gui`; `git fetch` e comparar os dois sentidos antes do push). O
  `[patch.crates-io]` de `vendor/iced_tiny_skia` é outro assunto (item mais abaixo, iced 0.15).

- [x] **CI de testes e lint** — feito em 2026-10-19: `.github/workflows/ci.yml` (fmt, clippy `-D warnings` e
  test por submodule, em push na main e PR). Falta só ver rodar no GitHub após o primeiro push.

- [ ] **Backup em git: o que falta para dar o plano por fechado (2026-11-03).** Fases 0–9 do
  `docs/plano-backup-estado-em-git.md` estão feitas e publicadas (v0.1.34: daemon 0.1.28, GUI 0.1.38; chiquitos já
  roda o daemon 0.1.28). Faltam, nesta ordem:
  1. **Ligar o sync no chiquitos**: PAT + repositório **privado**, começando em modo `commit`, depois `sync`.
     Hoje está `off`. Resolver antes a auditoria de valores em claro (`InfraAudit`) se aparecer algo.
  2. **Testar a restauração com Docker/Podman rodando como root** (VM ou servidor de teste): servidor limpo +
     `age_identity` = chave privada de recuperação + sync no mesmo repositório → `base` e `app` precisam ficar
     **Live** em ordem. O E2E de 2026-11-02 (R10.13) provou estado, secret e ordem, mas o healthcheck por IP não
     passa em podman **rootless**, então nunca viu Live.
  3. **Testes manuais do R10.7 3c**: rollback real de Compose, HTTPS+token contra GitHub/Gitea de verdade,
     procedimento de divergência por SSH.
  4. Opcional: comando para **recifrar os secrets antigos para a chave de recuperação** (os salvos antes de
     configurá-la, como o `REDIS_PASSWORD` do chiquitos, só abrem com o `age.key` do servidor).
  5. Ajustes visuais conhecidos: `<select>` da webui com fundo branco, "Cancelar" em português nos diálogos da GUI
     em inglês (embutido no glacier), desalinhamento leve nas linhas de env da webui com o botão 🔒.
  6. ~~`~/.local/share/rustploy/db` (banco dev antigo, alterado por um teste meu)~~ — apagado pelo usuário em 2026-11-03.

- [ ] **Infra as Code: trocar o "Exportar zip" do manifesto por "Baixar a infra" (zip dos YAMLs do `state_git`) e remover o resto (2026-11-04).**
  Hoje o IaC é um formato **paralelo** ao estado real: `ManifestExportAll` gera um `rustploy.yml` + `rustploy.vars.toml` (valores `Plain`
  redigidos para `${KEY}`, secrets como `secret:NOME`) e a GUI/webui montam um zip com isso (`manifest_zip.luau`, `webui/zip.js`).
  Depois que o estado passou a viver em YAML no git (`store::infra`, `projects/<id>/project.yml`, `settings.yml`, `providers.yml`),
  esse manifesto virou duplicação. O que o dono quer: um botão que **baixa a infra zipada**, e nada mais.
  - **Novo comportamento:** "Baixar infra" entrega um zip com **a mesma árvore que o `state_git` grava no repositório** (o que está em
    `store::infra`: `settings.yml`, `providers.yml`, `projects/<id>/project.yml`), com os segredos como `ENC[age:…]`, exatamente como
    no git — nada em claro, nada de formato inventado. Serve de cópia de segurança fora do GitHub e para inspeção. Decidir se o
    daemon monta o zip (um comando novo, ex.: `InfraDownload`, resposta com os bytes) ou devolve a árvore e o cliente zipa
    (como hoje); a primeira é mais simples para os dois clientes e evita zip em Luau e em JS.
  - **Onde fica o botão (decisão de 2026-11-04):** na própria aba **Sync** de Settings, abaixo do bloco de sync (é lá que a pessoa já
    pensa em "estado da infra no git"). **A aba "Infra as Code" deixa de existir** nos dois clientes (GUI e webui), sem substituta:
    sai a aba, o item de menu e a tela inteira, não só os botões de import/export.
  - **Upload não entra:** subir um zip de volta é, na prática, a operação de **sync** (apontar o repositório e sincronizar). Ao
    remover o import, a restauração a partir de arquivo vira "colocar a árvore num repositório e sincronizar" — documentar isso.
  - **Remover:** os comandos `ManifestApply`, `ManifestExport`, `ManifestExportAll` e `ManifestImport` (e as respostas `ManifestBundle`/
    `MissingEnvVars` que só eles usam) em `rustploy-shared`; os handlers e `features/iac/{manifest_apply,manifest_export,
    manifest_export_all,manifest_import}.rs`; a **aba Infra as Code inteira** nos **dois** clientes (campos `manifest*` do
    `webui/app.js`, `webui/zip.js`, `manifest_zip.luau`, a tela, o roteamento e a entrada de menu, mais as chaves de i18n que
    ficarem órfãs nos catálogos `locales/`); o botão novo entra na aba Sync; o trecho do `agent/manual.md`; e o guia
    `infra-as-code.mdx` do site (pt e en), que passa a descrever "Baixar infra" + sync. **Isto é uma quebra de protocolo:** sobe o
    `rustploy-shared` (junto com o rename do `db_kind`, se for na mesma leva) e agentes que usam `ManifestApply` pelo `rpc` perdem o comando.
  - **Cuidado — não confundir com o "pacote de serviço"** (`service_bundle`/`service_import`/`service_export`, copiar um serviço entre
    servidores): é outra funcionalidade, **fica**. Antes de apagar `features/iac/manifest.rs`, ver quem mais usa os tipos dele
    (`EnvDoc`, `ProjectManifest`…): `service_import.rs` e `service_bundle.rs` provavelmente dependem; o que for compartilhado
    muda de lugar em vez de ser apagado. A chave `db:` do manifesto (item do rename `db_kind` → `product`) some junto, o que **reduz
    o escopo daquele rename**: se este item sair primeiro, tirar `manifest.rs:209` da lista de lá.
  - **Fecho:** `grep -rIn "ManifestExportAll\|ManifestImport\|ManifestApply\|manifest_zip" .` vazio (inclui `docs/indice`, o site
    e o `rustploy-docs/dist` regenerado); `make index`; testes do `headless_tests.rs` que exercitam o import/export ajustados;
    verificar a pegadinha do cache imutável da webui ao testar (ctrl+shift+r).
- [ ] **Backup de volumes para S3, com restauração em poucos cliques (anotado em 2026-11-04, ainda sem projeto).** Só os **volumes**
  (dados de banco, uploads, o que o container grava): a configuração/infra **não** entra, porque já é sincronizada com o GitHub
  pelo `state_git`. Destino: qualquer S3 (AWS, Backblaze, R2, ou o próprio Storage MinIO/RustFS do rustploy em outra máquina).
  - **Requisito central: plug and play.** Servidor formatado → instalar o rustploy → informar o destino S3 + a chave de
    recuperação (a mesma `age_identity` do `state_git`) → **o rustploy restaura infra (git) e volumes (S3) sozinho**, na ordem
    certa (volumes antes de subir o serviço; `depends_on` já resolve a ordem entre serviços). Meta: poucos cliques, sem editar
    arquivo nem rodar comando. A restauração é o produto; o backup só vale se ela funcionar — testar com servidor limpo de verdade.
  - **Pontos em aberto, além dos cinco abaixo (2026-11-04):** (6) onde mora a configuração do destino S3 — como a restauração
    começa num servidor vazio, o destino e a credencial precisam estar no `state_git` (cifrados) ou ser digitados no primeiro
    acesso; decidir qual (ovo e galinha com a chave `age`); (7) quais volumes entram: só os nomeados do Compose, também
    bind mounts, e como ficam os volumes de serviços que o usuário não quer salvar (cache, `node_modules`); (8) o que fazer ao
    restaurar num servidor com **outro IP/domínio** (ingress, TLS, webhooks) e se os volumes já existentes são sobrescritos;
    (9) restaurar tudo de uma vez ("recuperar servidor") versus um serviço só ("voltar o banco ao backup de ontem"); (10) aviso
    de falha do backup (hoje não há canal de notificação) e verificação periódica de que o backup é restaurável;
    (11) custo/limite: tamanho máximo, compressão e o que fazer com volumes de dezenas de GB.
  - **Dump nativo de banco é o caso de uso principal, não um extra (decisão de 2026-11-04):** é a única tarefa recorrente que
    não vem do código do dono (migrations, seed, bucket, lint e testes já vêm dos repos dele, via `git_source` do job, e
    `chickie-setup`). Por isso o dump (`pg_dump`, `mongodump`, `mysqldump`, `redis-cli --rdb`) entra **como parte deste backup**,
    com restauração em poucos cliques, e **não** como "template de job" para o usuário montar. Templates de job foram avaliados
    e descartados como funcionalidade geral; reavaliar só se uma receita aparecer repetida em vários projetos.
  - **A decidir (perguntar ao dono antes de projetar):** (1) consistência — parar o serviço durante o backup, ou usar dump nativo
    para banco (`pg_dump`, `mongodump`…, já há o tipo do serviço e `managed_database`) e cópia crua só para volume comum;
    (2) agendamento e retenção por serviço (padrão sensato ligado para banco, desligado para o resto?);
    (3) cifrar os backups com `age` antes de subir (a chave já existe) — provavelmente sim, não deve ir nada em claro;
    (4) incremental/deduplicado (restic-like) ou tar completo por execução (mais simples, mais caro);
    (5) credenciais do S3 guardadas como secret cifrado, nunca em `config.toml` em claro.
  - **Pontos de contato:** `features/backup` (o antigo backup de env, aposentado na Fase 0 do plano de backup em git — não
    reaproveitar o nome sem ler o porquê), `features/jobs` (agendamento e execução one-shot), `state_git` (a restauração da
    infra, que deve ser o passo 1 do mesmo fluxo), Storage (RustFS/MinIO como destino), GUI + webui (os dois clientes).
  - **Teste obrigatório de aceitação:** criar postgres + app com volume, gravar dados, fazer o backup, formatar/subir servidor
    limpo, restaurar pelo fluxo da UI e conferir que os dados voltaram. O item "Testar a restauração com Docker/Podman rodando como
    root" do backup em git acima é o mesmo ambiente de teste — fazer os dois juntos.
- [ ] **GUI e webui: abrir Settings deve disparar sozinho o "Refresh git list" (2026-11-04).** Hoje, ao ir direto para a aba **Sync**, a
  lista de contas Git fica vazia até a pessoa voltar à aba **Git** e atualizar a lista à mão. Ao clicar em **Settings**, os **dois**
  clientes (GUI e webui) devem disparar a mesma ação do botão "Refresh git list", para que a Sync já encontre a lista carregada,
  em qualquer ordem de navegação.
- [ ] **i18n (GUI e webui): o que ficou de fora da primeira versão (2026-10-31).** Inglês padrão + pt-BR a um clique
  já valem nos dois clientes (ver "Idiomas" no `AGENTS.md`). Pendências: (1) texto que o código grava em estado
  (status, toast, `ctx.*_message`) fica no idioma em que nasceu — só a linha de status do login foi tratada;
  (2) rótulos da bandeja (`tray`) seguem em inglês (menu nativo montado uma vez); (3) as mensagens de erro do
  **daemon** (a API) não são traduzidas; (4) sem plural nem formato de número/data por idioma na webui (a GUI tem
  `i18n.num`/`i18n.date`); (5) um terceiro idioma pede só os dois catálogos novos (`locales/<idioma>.json` e
  `webui/locales/<idioma>.js`) e a entrada no seletor; (6) mensagem montada por concatenação de pedaços exige
  a mesma ordem de palavras nos idiomas — trocar por mensagens inteiras com `{n}` onde a ordem importar.

- [ ] **Dependências entre serviços + autostart no boot.** Plano de 2026-07-11 em
  `docs/plano-dependencias-e-autostart.md`, nunca executado (não há `depends_on` nem `autostart` no
  daemon). Depois de um reboot da VPS a ordem de subida (banco antes da app) depende do `restart:` do
  container. Os passos que citam o TUI estão obsoletos; a UI vai nos dois clientes (GUI e webui).

- [ ] **Persistir o log de runtime dos containers.** O swap de deploy destrói o container antigo e o
  log dele morre junto, justamente quando mais se precisa dele. Maior valor e maior esforço entre os
  buracos conhecidos; contexto em `docs/plano-erro-de-deploy-invisivel.md`.

- [ ] **Exportar e importar DADOS de um container ou volume, para subir em outro banco/servidor.**
  O export atual (`rustploy.yml` + `rustploy.vars.toml`) leva só a configuração: volumes e bancos
  ficam de fora, então não dá para reproduzir a infra de uma VPS em outra máquina (nem testar uma
  migração com dados reais a partir do zip). Achar uma forma de baixar os dados e subi-los em outro
  lugar. Pontos a decidir:
  - **Volumes (qualquer coisa):** exportar um volume nomeado como `.tar.gz` (container descartável
    montando o volume: `tar czf - -C /data .`) e importar noutro volume/servidor; pela API do
    rustploy (stream) e pela GUI/webui ("Baixar volume" / "Enviar para volume"). Parar o serviço
    (ou usar snapshot) para o dado ficar consistente.
  - **Bancos (preferir dump lógico a copiar o diretório):** `pg_dump`/`pg_restore` (Postgres),
    `mysqldump`/`mysql` (MySQL/MariaDB), `mongodump`/`mongorestore`, `redis-cli --rdb`/RDB; escolhendo
    o engine pelo `db_kind` do serviço. Existe a migração entre servidores em `migration.rs`
    (`docker run ... pipe_script`) para bancos: reaproveitar, e estender para "exportar para arquivo"
    e "importar de arquivo", além de servidor -> servidor.
  - **Banco compartilhado** (`infra/pg-central`, `shared: true`): exportar/importar por banco/usuário.
  - Incluir no export da infra um modo "com dados" (opcional, pesado) ou um bundle à parte, com
    aviso de tamanho e de segredos; `secret:NOME` nunca é decifrado (regra do `ServiceBundle`).
  - Depois disso, o teste de migração Docker <-> Podman pode usar dados reais exportados da VPS.

- [ ] **Testar o backend Podman no que ficou de fora do smoke test** (o usuário vai
  fazer, 2026-10-18; detalhes em `docs/plano-backend-podman.md`, seção
  "Pendências"): upload de zip (Archive); TLS/ACME com domínio real; volumes e
  dados de bancos; jobs one-shot e registry embutido; subir os serviços reais (todos
  `Stopped`); `install.sh --engine docker` e `--engine podman` de ponta a ponta numa
  VPS limpa, depois da release.

- [ ] **Depois de migrar a VPS, conferir**: (1) `reboot` e ver se os containers com `restart:` voltam
  (`systemctl is-enabled podman-restart.service`); (2) de dentro de um container, `curl` para fora
  (saída pelo netavark/ufw); (3) portas publicadas pelo Compose (Supabase) e as `host_port` 20000-20004,
  de fora da VPS; (4) o que estava no `/etc/docker/daemon.json`.

- [ ] **Migração Docker -> Podman com dados reais** (`install.sh`/`make install-rustploy`): só
  foi testada com stubs. Rodar primeiro com `--dry-run`; o backup fica em `/var/backups/`.

- [ ] **Testar em máquinas de verdade o que só rodou em container.** (1) `install.ps1` num Windows real,
  principalmente o atalho do Menu Iniciar e o aviso do SmartScreen; (2) o `.deb` arm64 numa VPS arm64;
  (3) `install.sh` numa VPS com systemd (subir `rustployd` e o `rustployd-fw.socket` e confirmar com
  `systemctl status`). Detalhes em `docs/release.md`, seção "Ainda sem teste real".

- [ ] **`ServiceUpdate` é substituição total, não patch.** Um round-trip mal feito apaga configuração
  sem aviso. Oferecer atualização parcial ou, no mínimo, avisar/recusar quando campos existentes
  somem do payload.

- [ ] **Auditar `unwrap()` fora de teste no daemon**, começando por `features/iac/*`
  (`service_import.rs` 25, `manifest_import.rs` 25, `manifest.rs` 18) e `features/deploy/queue.rs`
  (18). São os maiores arquivos e lidam com entrada externa (manifestos importados): trocar por erro
  propagado. Avaliar também quebrar `manifest.rs` (1260 linhas) e `service_import.rs` (1013).

- [ ] **Registry embutido: integrar com o executor de deploy e push-to-deploy.** Fases 1 e 2 (auth e
  ingress), sub-aba da GUI e GC já estão prontos; falta isso. Ver `docs/plano-registry-embutido.md`.

- [ ] **Comando para remover/parar um container solto pela API.** `ServiceDelete` já para e remove os
  containers do serviço (corrigido); mas `RemoveContainer` (aba Containers) continua sem `force` e não há como
  parar um container que não pertence a um serviço.

- [ ] **A web UI segura o CSS/JS antigo depois que o daemon atualiza.** Visto em
  2026-10-16: o servidor já servia o `app.css` novo (54055 bytes, conferido por
  `curl`), mas o navegador continuou com o antigo mesmo depois de desregistrar o
  service worker e apagar os caches. Hipótese, não confirmada: `sw.js` faz
  `cache.addAll()` do app shell pelo cache HTTP, que é `immutable` por 1 ano, e as
  URLs não levam hash (`/app.css`, `/app.js`). Olhar `webui/sw.js`, o
  `Cache-Control` em `src/api/web_ui.rs`/`build.rs` e se faz sentido versionar as
  URLs dos assets (ou o SW buscar com `cache: 'reload'`). Hoje o remédio é
  Ctrl+Shift+R, às vezes mais de uma vez, e o texto de `interface/web-ui` já avisa.

- [ ] **Schema do protocolo.** A lista de comandos do manual do agente (`api/agent/manual.md`) é
  curada à mão e pode divergir dos `Command` reais. Gerar com `schemars` sobre a crate
  `rustploy-shared` (exige derivar nos tipos do protocolo e publicar o shared).

- [ ] **Migração Podman -> Docker.** Hoje só existe Docker -> Podman; voltar ao Docker é manual
  (o `install.sh` recusa: "voltar ao Docker não é automático"). Fazer o caminho inverso com as mesmas
  garantias: backup, `podman save | docker load` das imagens (antes de parar nada), cópia de TODOS os
  volumes (`cp -a`; opções NFS/bind recriadas sem copiar), redes de usuário/compose, parar o
  `rustployd` e os `rp_*`, rollback se falhar, trocar `[docker] socket_path` para
  `/var/run/docker.sock`, remover os drop-ins do Podman (`rustployd.service.d/podman.conf` e
  `podman.socket.d/rustploy.conf`), reiniciar e refazer o deploy dos serviços que estavam no ar. O
  esqueleto de `migrate_to_podman` em `install.sh` serve de base (espelhar `docker`/`podman`). Lembrar
  que o `podman-restart.service` e os arquivos de `/etc/containers/*.conf.d/rustploy.conf` não têm
  equivalente no Docker (o `dockerd` já religa e já tem docker.io como padrão), e que os
  nomes de imagem voltam a ser curtos.

- [ ] **Assinar o `.exe` do Windows** (SignPath, tier OSS): o job `sign-gui-windows` é pulado enquanto a variável de
  repositório `SIGNPATH_ORGANIZATION_ID` não existe. Passos em `docs/windows-code-signing.md`.

- [ ] **Capturar o diálogo "Remover job?" do app desktop e usar em
  `interface/schedules` (pt e en).** O desktop passou a pedir confirmação ao
  remover um job (`rustploy-gui`, `handlers/jobs.luau::job_delete`, commit
  `05bd708`), e a página já diz isso, mas sem imagem. Só dá para capturar depois
  de compilar e instalar o app novo. Borrar host, projeto e nome do job. Depois,
  refazer o deploy do site (`wrangler deploy` em `rustploy-docs/`). A confirmação
  em si nunca foi exercitada numa janela real (só os testes de renderização
  passaram): conferir Cancelar (painel ⋯ continua aberto) e Remover.

- [ ] **Resto do plano do site (`docs/plano-site-documentacao.md`).** Falta a
  captura de instalar a web UI como PWA (a barra do navegador não sai na
  extensão do Chrome: tem que ser tirada à mão), o CI de qualidade do site
  (links, exemplos executáveis, Lighthouse) e o domínio definitivo (canonical,
  sitemap e `Sitemap:` do `public/robots.txt`).

- [ ] **Remover o `vendor/iced_tiny_skia` quando o iced 0.15 sair.** O
  `text_editor` (`<textarea>`) vazava a linha parcial da borda no renderer de
  software; o upstream corrigiu na `master` (commit `23170119b`, 2026-01-28)
  mas nenhuma release 0.14.x tem. Conferir `cargo search iced` / crates.io.
  Quando houver release com o commit: (1) glacier-ui migra `iced` e publica;
  (2) rustploy sobe o `glacier-ui`; (3) no repo `rustploy-gui`, apagar `vendor/iced_tiny_skia/`,
  `vendor/README.md` e o `[patch.crates-io]` do `Cargo.toml`; (4) abrir a
  janela de logs e conferir que o recorte continua certo. Contexto em
  `vendor/README.md`.

- [ ] **Tirar o glacier-ui de dentro do iced (proposta, nada feito).** Dois caminhos, o A primeiro:
  (A) o glacier-ui passa a mandar no layout (Taffy) e nos eventos, e o iced vira só o backend de desenho e
  entrada; (B) depois, spikes de outros backends (Vello+Parley, Slint, egui) atrás do mesmo `trait Backend`.
  Fases, medições e decisões em aberto: `docs/plano-evolucao-do-backend-grafico.md`.

- [ ] **Página "Para quem vai contribuir" no site de docs.** Última seção do
  `docs/plano-site-documentacao.md` (§4, item 15) que não existe. Adiada de
  propósito: o projeto continua **fechado a contribuições** por enquanto (decisão
  de 2026-10-16). Quando retomar, a página só condensa a Parte 3 do `AGENTS.md`
  (arquitetura, build, testes, convenções) e aponta para ele, que continua sendo a
  fonte. Não copiar o texto inteiro, para não envelhecer em paralelo.

- [x] **Reorganizar o daemon e o `shared`** — feito em 2026-10-18 (`docs/plano-reorganizacao-daemon-e-shared.md`,
  seção "Como ficou"). O passo seguinte (contextos por feature) e a `web_ui` também já foram feitos.
- [ ] **Deploy antigo aparece `Stopped` enquanto o container ainda roda** (visto no Chiquitos, 2026-11-04): ao entrar um deploy novo na fila/construção, o registro do
  deploy `Live` anterior vira `Stopped` em segundos (api 05:07:35, 14 s depois; worker 25 s depois), mas o container antigo só é trocado quando o novo
  fica pronto (até ~25 min depois) e o status do serviço segue `Running`. A GUI/webui mostram o serviço "parado" nesse intervalo. Não é o `stop.rs`
  (só roda no parar manual); achar o caminho que faz a transição e manter `Live` até a troca (ou usar `Pruning`, "substituído").
