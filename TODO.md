# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

Ordenada por valor (o mais valioso primeiro, avaliação de 2026-10-19). Ao
acrescentar um item, encaixe-o na posição certa em vez de colar no fim.

- [x] **Publicar o `rustploy-shared` 0.16.0 e tirar o `path` do daemon** (decisão de 2026-11-03: até lá `cargo install
  rustploy` entrega a 0.1.28 e o `rustploy-daemon` não compila sozinho). Passos em `docs/release.md` (v0.1.35).
- [x] **Publicar o app 0.1.41 (release v0.1.41)** — feito em 2026-11-06: crates.io, tag, release assinada (5 arquivos + 4 `.minisig`) e `install.sh gui` conferido num `debian:12`
  (app 0.1.41-1). **Ainda não visto:** o botão **Carregar anteriores** da janela de logs, que só aparece com mais de 500 linhas guardadas num serviço que não seja Compose;
  conferir quando algum serviço passar disso. O `install.ps1` e o daemon (0.1.34, inalterado) não foram retestados.
- [ ] **Achado de 2026-11-07 (Chiquitos, após subir o daemon 0.1.36):** no boot, `worker` falhou com "não ficou pronto em 120s" e `api`/`ui` com "dependência não ficou pronta"; minutos depois os três estavam `Running` sozinhos. Investigar a janela de 120s do autostart quando o worker tem de esperar broker/db recém-recriados. RabbitMQ: o broker foi recriado hoje às 14:04; o log do container novo ainda não tem a janela de 3 min do api, então a validação do conserto do IP fica para o próximo deploy da api.
  **Causa e correção (2026-11-07, sem commit):** o `worker` tem `healthcheck: None` e porta 80, mas não escuta porta. O deploy e o watchdog tratam `None` como "nada a testar"; o `wait_ready` do autostart forçava TCP e falhava após 120s em todo boot. Agora, esgotado o prazo com `None`, container rodando basta (`features/deploy/autostart.rs`). Falta publicar o daemon e ver um boot real no Chiquitos.
- [x] **Publicar o que estava pronto (2026-11-07)** — feito: `rustploy-shared` 0.20.0, `glacier-ui` 0.151.0/0.152.0, `rustploy` 0.1.37 e `rustploy-gui` 0.1.44 no crates.io; release **`v0.1.45`** assinada e publicada (conferida: checksums, assinaturas, `install.sh gui` num `debian:12`); docs no ar. **Continua não visto rodando:** a edição no lugar (app e web UI), o campo que rola e o toast no app, o push-to-deploy num servidor real (`docker login` → `docker push` → redeploy), o boot do Chiquitos sem o erro de 120s do worker, e o `install.sh` do daemon / `install.ps1`. **Ao atualizar o Chiquitos e o Itemize** (o Itemize ainda está no daemon 0.1.20): conferir `product` nos bancos depois do primeiro boot.
- [x] **`EnvValueSet`: trocar só o valor de uma variável, no projeto ou no serviço** — feito em 2026-11-07 (sem commit/publicação): `shared` 0.20.0 (`Command::EnvValueSet`), `platform/secrets.rs::set_env_value` (atômico, mantém a posição; recusa chave inexistente, a máscara `***` e secret que não existe no escopo), handler `env_value_set.rs`, dispatch, manual do agente (o MCP já alcança pelo `rpc`; teste fixa que não é proibido ao agente). 2 testes de daemon + 1 de protocolo.
- [x] **Edição no lugar da chave e do valor das variáveis (GUI e webui)** — feito em 2026-11-07 (sem commit/publicação). O ✏️ não leva mais a variável para um formulário no topo: a chave vira um campo **na própria linha**, e o novo ✎ faz o mesmo com o valor (variáveis do projeto e do serviço). Enter/✓ grava (`EnvKeyRename` / `EnvValueSet`), ✕ desiste; na webui clicar fora também grava e Esc desiste. Numa variável secret o valor editável é a referência `secret:NOME`, nunca o valor cifrado. GUI: `handlers/secrets.luau` (`*_env_edit_key/value`, `env_edit_apply/cancel`), `Helpers.env_value_from_draft`, `project_env.gv`/`environment.gv`, 8 testes Luau; webui: `envEdit` em `project_detail.js`/`service_detail.js`, `envValueFromDraft`, `.env__inline`. **Não visto rodando** (GUI e webui); a GUI depende do `glacier-ui` 0.152.0.
- [x] **`glacier-ui`: `textinput(autofocus = true)`** — feito em 2026-11-07 (0.152.0, sem commit/publicação): o campo de uma linha pega o foco ao aparecer, com o texto selecionado; visto num exemplo digitando sem clicar. Base da edição no lugar.
- [x] **Campo de texto rola na horizontal e toast animado** — publicados no `glacier-ui` **0.151.0** (2026-11-07) e usados no `rustploy-gui` (commit `9ad5933`, enviado): a janela do campo acompanha o cursor (digitar, colar, setas, seleção) e o toast sobe, tem a barra de tempo lateral e desliza para fora. Vistos em um exemplo; **falta ver no app do Rustploy**.
- [x] **Aba Variáveis do projeto (GUI): espaço da lista, Add cortado, Salvar secret** — feito em 2026-11-07 (commit `9ad5933`, enviado): a aba inteira rola, o formulário de adicionar ocupa uma linha, o último botão deixou de ser cortado pela barra de rolagem (folga de 16px em `.env--flow`) e o "Salvar secret" ficou ao lado do campo de valor. **Falta ver no app.** A aba Environment do serviço ainda tem a lista com rolagem própria (mesmo aperto) — não mexi.
- [x] **Publicar `glacier-ui` 0.152.1 e daemon 0.1.38 (2026-11-07, depois da v0.1.45)** — feito: os dois no crates.io e na `main`; release **`v0.1.46`** assinada e publicada (checksums, assinaturas e `install.sh gui` num `debian:12` conferidos). `glacier-ui` 0.152.1 corrige 48 testes desatualizados e o bug real dos diálogos com corpo (`pedir_texto`/`escolher_cor`/progresso, quebrados desde a 0.149.0). Daemon 0.1.38: `reconcile` não condena o `Live` no boot. **Falta conferir no Chiquitos** (depois de atualizar o daemon para 0.1.38): o deploy da api e o do worker voltam a `Live` e um reinício do daemon não os derruba. O app segue na 0.1.44 (com `glacier-ui` 0.152.0).
- [ ] **Limites do agente (MCP): fechar o que ele consegue fazer e não deveria, e liberar o que deveria conseguir** — plano em `docs/plano-limites-do-agente-mcp.md` (2026-11-07). Achado principal: as restrições do agente (não revelar secret, mascarar senhas, comandos proibidos) valem **só no `/mcp`**; o `POST /api/rpc` com o mesmo token não as aplica, então hoje o token é acesso total. **Feito:** (A) a credencial de provider só vai para o host do provider (clone de serviço, de job e de aquecimento, e o `state_git`; recusa já na configuração do `remote`); daemon 0.1.39. **Falta, nesta ordem:** (C) `JobUpdate` restaurar variáveis mascaradas (hoje o agente não edita um job com senha em claro; é um defeito); (0) papéis por token — **decisão do dono:** a chave atual vira a de **visitante** e a **mestra** é opcional (`rustployd access-key --master`, grava `[api].master_token`); gerar a mestra é o que liga a proteção, e sem ela nada muda; as regras passam a valer no `/mcp` **e** no `/api/rpc`, a mestra nunca sai pela API, `GET /api/events` recusado ao visitante, GUI e webui entram com a mestra; (B) `InfraAuditAllow` proibido ao visitante (só vale depois do 0); (D) níveis do visitante (observador, operador, administrador) em vez de varrer bind mounts e compose. **Em aberto (seção 10 do plano):** três níveis ou um? `credentials` (secret) com URL fora dos providers (recusar ao visitante ou aceitar?); o visitante configura o `state_git`?; o visitante cria serviços novos?
- [x] **Release `v0.1.47` (daemon 0.1.39)** — publicada, assinada e conferida em 2026-11-07 (checksums, assinaturas, `install.sh gui` num `debian:12`, docs no ar). **Falta confirmar num servidor real:** um clone de repositório privado com o provider depois da mudança do host da credencial, e o `state_git` contra um GitHub de verdade. Atualizar o Chiquitos para o daemon 0.1.39 (que já inclui o 0.1.38 com o conserto do `reconcile`) e conferir que o deploy da api e o do worker voltam a `Live`.
- [ ] **Chave S3 própria por app** no Storage (hoje a app usaria a chave root) e rotacionar a senha do `db` do Chiquitos
  (foi exibida a um agente antes da Fase A). Ver `docs/plano-credenciais-cifradas-e-storage.md`.
- [x] **Template `minio` do catálogo** aponta para `minio/minio` (não puxável): trocar a imagem no blueprint.
- [x] **Release com o `rustploy-podman-ufw`** (commitado no daemon, ainda sem release): só então fazer o deploy das docs (a
  seção "Podman e ufw" e a receita de bucket com `curl` já estão commitadas, mas o site ainda tem a receita errada com `aws-cli`).
  (O isolamento entre dois projetos já tem teste de laboratório; falta só rodar num servidor real com dois projetos.)
- [x] **Aposentar o `db_kind`: renomear para `product` — código feito em 2026-11-05 (sem commit); falta publicar e testar em servidor.** Feito: campo, GUI Luau, webui, docs, índice; migração do schema 1→2 na leitura (`upgrade_project_v1`) e em `Repo::open` (com backup `infra.bak-schema1`), SQL antigo em `migrate.rs`, `db:` do manifesto aceito como alias de `product:` (tirar na release seguinte). `shared` está em 0.18.0 e o daemon já pede `0.18.0`: **não compila até publicar o `rustploy-shared` 0.18.0** (os testes rodaram com `--config patch.crates-io` na linha de comando, nada gravado). Faltam: publicar shared → daemon → GUI na mesma release; smoke em servidor de teste (postgres, kafka, rustfs → aba Connection); conferir `grep -rn db_kind` na infra dos dois servidores depois do primeiro boot. Referências que ficam de propósito: código de migração e o `importer` (usa shared 0.5.0). **Marcado como concluído a pedido do dono em 2026-11-05; ainda a verificar após a release:** publicar shared 0.18.0 → daemon → GUI, smoke em servidor de teste e `grep db_kind` na infra dos dois servidores e no repositório do GitHub. Texto original abaixo. O campo nasceu quando só havia
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
- [x] **Webhook: o daemon deve aceitar os dois formatos de corpo, `application/json` e `application/x-www-form-urlencoded`** (`payload=<json>`,
  com percent-decoding; escolher pelo `Content-Type`). O padrão da UI do GitHub é `form`, e hoje nele o `watch_paths` e o filtro de branch são
  ignorados em silêncio (deploya tudo); só funciona com `content_type: json`. Ajustar `extract_push_branch`/`extract_changed_files` em
  `api/http/public_routes.rs` (já existe um parser de form no arquivo), com testes dos dois formatos, e atualizar o guia de webhooks (pt/en).
  Considerar também ignorar o `ping` do GitHub, que hoje dispara um deploy.
  **Feito em 2026-11-05** (`json_body` + `ping` ignorado em `public_routes.rs`, testes, guia pt/en); falta commit/release e deploy das docs.
- [x] **Sem registro de deploy da api e do worker após a atualização para 0.1.33** (Chiquitos, 2026-11-05) — **causa achada e corrigida em 2026-11-07 (sem commit/release).** Conferido no Chiquitos (daemon já no 0.1.37): `DeployHistory` da api e do worker voltam cheios e o `live_container_id` de ambos está preenchido, mas o deploy **mais recente de cada um está `Stopped`** ("container not running at read time") com o serviço `Running`. **Causa:** o `reconcile` (`api/handlers/deploys/reconcile.rs`) roda a cada snapshot e a cada `DeployHistory` e, com o container parado, condenava o `Live` para `Stopped` **e** chamava `update_status(Stopped, None)`, que **apagava o `live_container_id`** — inclusive na janela do **boot do daemon**, antes de o autostart religar o container (a leitura das 14:07 de 2026-11-07, com o worker ainda em erro, fez exatamente isso). Depois o container voltava, mas nada reerguia o deploy: ficava `Stopped` para sempre. É a mesma raiz do item "Deploy antigo aparece `Stopped`…" mais abaixo. **Correção:** (1) o `Live` é **poupado** quando a intenção é "no ar" (`desired_state = Running`) e o daemon subiu há menos de 10 min (`BOOT_GRACE`); (2) o deploy mais recente marcado `Stopped` **pelo reconcile** (e não por um "Parar" de uma pessoa, mensagem diferente) volta a `Live` ("container running again") quando o container está rodando — o que conserta também o que já está gravado em produção, na primeira leitura depois da atualização. Testes das duas regras (puras); o caminho com Docker não tem teste automatizado. **Falta:** publicar o daemon (0.1.38) e conferir no Chiquitos que o deploy da api e do worker voltam a `Live` e que um reinício do daemon não os derruba mais.
- [x] **Migrations embutidas no `chickie-setup`** (`include_dir!` + `build.rs` com `rerun-if-changed=migrations`) em vez de ler `.sql` do disco; tira o `COPY migrations` da imagem.
- [x] **Redeploy do worker e do scheduler** com o código novo (Fase G: a api já foi verificada) e **ajustar a receita de bucket nas docs** (ainda ensina `curl`).
- [ ] **RabbitMQ: "no heartbeat received from server for too long"** no log da api (2 erros 2 min após o boot, 2026-11-04 04:23); investigar se é pré-existente.
  **Investigado em 2026-11-05 (logs do Chiquitos via MCP): é pré-existente** — já em 2026-11-04 (05:02, 05:38) e de novo em 03:38 e 07:54 de hoje. Padrão fixo: **a 1ª conexão AMQP de todo processo Chickie recém-iniciado (api ×2, worker, scheduler) cai a 2 min no cliente ("no heartbeat received from server for too long") e o broker a fecha aos 3 min ("missed heartbeats from client, timeout: 60s")**. Worker/scheduler reconectam em 5 s e a 2ª conexão dura horas (houve uma de 22 h); a **api não loga reconexão** e o broker não mostra conexão nova dela depois, então os publishers da api podem ficar mortos até o próximo restart (não confirmado: nada publicou no intervalo). Pista a seguir: o worker aparece no broker com IPs diferentes no mesmo container (`.84` na 1ª conexão, `.85` na reconexão) — sugere troca/reanexo de rede no `SwappingIn` invalidando a conexão aberta no staging. **CONFIRMADO em teste local (2026-11-05, podman + RabbitMQ 4 + cliente AMQP):** o `attach_network_alias` (`engine/containers/lifecycle.rs`) faz `disconnect_network(force)` + `connect_network`, o container ganha **IP novo** (.8 → .10) e toda conexão aberta no staging fica sem tráfego; o cliente leva `HeartbeatTimeout` em ~130 s e o broker loga `missed heartbeats… duration: '3M, 0s'`, idêntico ao Chiquitos; o controle sem troca ficou vivo. **Religar com o mesmo IP (`--ip`) manteve a conexão viva por 213 s+.** Conserto no rustploy: guardar o IP antes do `disconnect` e reconectar com `ipam_config.ipv4_address` igual. **Bug separado no Chickie:** `RabbitMQTaskPublisher::publicar` (`crates/api/src/infrastructure/rabbitmq_task_publisher.rs`) só recria o canal quando `Option<Channel>` é `None`; canal morto continua `Some`, o publish falha e o canal nunca é descartado — a api não publica mais nada até reiniciar. Não testei publicar no Chiquitos (nada de tarefa real disparada).
  **Corrigido em 2026-11-06 (commitado e enviado):** (1) rustploy — `attach_network_alias` reconecta pedindo o **mesmo IPv4** (com fallback para IP novo se o engine recusar); teste `alias_mantem_o_ip` (`#[ignore]`, engine real; falha sem o conserto, passa com ele; no Podman rootless precisa de um 2º container na rede; `RP_TEST_SEM_COMPANHEIRO=1` testa o container sozinho). **Validado em Podman rootful 5.7.0 (2026-11-06) pelo socket de compatibilidade: passa com e sem o 2º container, e sem o conserto falha ("o IP mudou na reconexão"), então o IP muda no rootful também. (2) Chickie (`../chickie`, branch `main`, árvore já tinha mudanças suas): `RabbitMQTaskPublisher::publicar` descarta o canal desconectado/com erro e reconecta, 1 nova tentativa; teste `canal_morto_e_recriado_no_proximo_publish` (`TEST_AMQP_URL`, `--ignored`) falha no código antigo com `invalid channel state: Closed` e passa no novo. **Falta:** validar no Chiquitos depois do deploy do daemon novo (no próximo deploy da api, o broker não deve mais logar `missed heartbeats` aos 3 min); o publisher só precisa ser commitado na `main` do chickie (as branches `deploy/*` não são mais usadas desde o `watch_paths`).
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
  1. ~~**Ligar o sync no chiquitos**: PAT + repositório **privado**, começando em modo `commit`, depois `sync`.~~ — **feito**
     (conferido em 2026-11-05): modo `sync`, repositório privado `chickie-infra`, último push OK às 03:00 UTC, `ahead/behind` 0,
     `blocked_by_plaintext` 0, `recovery_configured: true`, sem pausa nem erro.
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

- [x] **Infra as Code → "Baixar a infra"** — feito em 2026-11-05 (release v0.1.40): `InfraDownload`/`InfraFiles` (shared 0.17.0), botão na aba Sync da GUI e da webui,
  aba "Infra as Code" e comandos `Manifest*` removidos, docs reescritas (guia `infra-as-code`, referência do formato do serviço), `InfraDownload` testado num daemon real
  (devolve os 4 arquivos; recusa com `PlaintextInInfra` quando há senha em claro).
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
- [x] **GUI e webui: abrir Settings deve disparar sozinho o "Refresh git list" (2026-11-04).** Hoje, ao ir direto para a aba **Sync**, a — feito em 2026-11-05 (webui: `nav('settings')`; GUI: `nav_settings`/`state_git_open`); aguarda a próxima release.
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

- [x] **Dependências entre serviços + autostart no boot** — conferido em 2026-11-05: `depends_on` e `autostart` existem no `ServiceSpec`, há `deploy/autostart.rs` e os campos aparecem na GUI (`views/service/advanced.gv`) e na webui (`service_detail.js`); o texto abaixo é o original (desatualizado). Plano de 2026-07-11 em
  `docs/plano-dependencias-e-autostart.md`, nunca executado (não há `depends_on` nem `autostart` no
  daemon). Depois de um reboot da VPS a ordem de subida (banco antes da app) depende do `restart:` do
  container. Os passos que citam o TUI estão obsoletos; a UI vai nos dois clientes (GUI e webui).

- [x] **Logs por execução: botão "Logs de execução" + lista das execuções passadas, com janela por registro, excluir e limpar selecionados** — implementado em 2026-11-06 (sem commit publicado): `log_store` com pasta por execução + `meta.json` + migração dos segmentos da v0.1.40, `LogsExecutions`/`LogsExecutionDelete` (`rustploy-shared` 0.19.0), `LogsHistory` por `container_ids`, webui e GUI. Como foi feito e verificado: `docs/plano-log-de-runtime-persistido.md` §7. **Falta:** publicar `rustploy-shared` 0.19.0 → daemon → app e fazer o deploy das docs (já escritas); **ver a janela nativa da GUI** (e a recusa da execução atual com um container `live` real) num servidor com a versão nova; conferir a migração numa cópia dos logs do Chiquitos antes do primeiro boot em produção.
- [x] **Persistir o log de runtime dos containers** — feito em 2026-11-05 (daemon: `platform/log_store.rs`, `LogsHistory`, `[logs]`; webui e GUI com
  "Carregar anteriores"). Plano e como foi verificado: `docs/plano-log-de-runtime-persistido.md`. **Pendências deste item:** (1) a troca (`Promoting`) com captura do container antigo **foi vista em produção** (Chiquitos, 2026-11-05: o log do container antigo continuou na janela,
  com o separador de container); o rollback e o stream contínuo também foram verificados; (2) a janela de logs da **GUI nativa** abriu e mostrou o histórico (2026-11-05), mas o botão `log_window_load_older` ("Carregar anteriores") ainda não foi exercitado de verdade;
  (3) logs de serviços **Compose** seguem só ao vivo. (Docs do site: feitas.)
- [ ] **Exportar e importar DADOS de um container ou volume, para subir em outro banco/servidor.** **Restrição de design do dono (2026-11-07), a respeitar antes de tudo:** o Rustploy **não lida diretamente com containers**; lida com eles **por meio dos serviços**. Então a exportação e a importação têm de ser operações **de um serviço** (um `service_id`, no mesmo molde dos outros comandos: `ServiceDataExport`/`ServiceDataImport` ou nome equivalente), que resolvem sozinhas quais volumes e qual banco pertencem ao serviço, e **não** receber um id de container nem um nome de volume solto. **Nenhum design nem código foi feito ainda: o próximo passo é escrever o plano em `docs/` (com contexto, porquê e exemplos), antes de implementar.** Texto original abaixo.
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

- [x] **`ServiceUpdate` é substituição total, não patch** — feito em 2026-11-07 (sem commit): `/api/rpc` recusa (400) um `ServiceUpdate` cujo payload omite campos que o serviço já tem preenchidos; limpar de propósito = mandar a chave vazia. GUI e webui já devolvem o spec completo. **Fora do alcance:** o MCP/agente (`tools.rs`) recebe `Command` já decodificado, sem como saber o que foi omitido; segue só o aviso do manual. Atualização parcial de verdade (patch) não foi feita.

- [x] **Auditar `unwrap()` fora de teste em `features/iac/*` e `deploy/queue.rs`** — feito em 2026-11-07. Os números do item (25/25/18/18) contavam código de teste: em produção havia **1** em `iac/` (`parse_mem`, que também estourava o `u64` com `99999999999g`: agora `checked_mul`, testado) e **11** em `queue.rs` (9 `lock().unwrap()` viraram `DeployQueue::lock`, que sobrevive a mutex envenenado — testado; os 2 `expect` do semáforo ficam, com a razão no texto: ele vive na própria fila e nunca é fechado). `manifest_import.rs` não existe mais (virou `service_import.rs`, sem nenhum). **Não auditei** o resto do daemon.

- [x] **Registry embutido: push-to-deploy** — feito em 2026-11-07 (sem commit). O pull autenticado pelo executor (Fase 3) já existia; faltava a Fase 4: `ServiceSpec.registry_auto_deploy` (opt-in) e `Event::RegistryPush` (shared 0.20.0), gancho no `PUT /v2/<repo>/manifests/<tag>` (`RegistryStorage::notify_push`/`subscribe_pushes`) e `features/registry/push_deploy.rs`, que redeploya os serviços cuja imagem aponta para o registry embutido e casa **repo e tag** (testes do casamento e do aviso de ponta a ponta no `push_pull_fluxo_completo`). Toggle na GUI (aba Advanced) e na webui. **Falta:** smoke real (`docker login` + `docker push` num servidor com o registry ligado e ver o redeploy), publicar, e o export de serviço (`ServiceBundle`) não leva o campo ainda (o `autostart` também não leva).

- [x] **Comando para remover/parar um container solto pela API** — conferido em 2026-11-07: `StopContainer` já existe (recusa o de serviço vivo, para o órfão) e `RemoveContainer` remove o parado, na GUI e na webui. Fica **sem `force` de propósito**: container rodando é recusado pelo Docker (para-se antes).

- [x] **A web UI segura o CSS/JS antigo depois que o daemon atualiza** — causa confirmada e corrigida em 2026-11-07 (sem commit): o `cache.addAll()` do `sw.js` buscava pelo cache HTTP `immutable` e enchia o cache novo com bytes velhos; agora usa `cache: "reload"`. **Falta** publicar e ver, num navegador com a versão antiga instalada, que a atualização aparece sem Ctrl+Shift+R.

- [x] **Schema do protocolo** — feito em 2026-11-07 como **teste** (sem `schemars` nem publicar o shared): `api/dispatch.rs::manual_do_agente::todo_comando_esta_no_manual_do_agente` extrai os nomes do `match` exaustivo do `dispatch` e exige cada um em `agent/manual.md`. Na primeira rodada achou 15 comandos sem documentação (StateGit*, UpdateStatus/ApplyUpdate/UpdateRollback, ServiceWarm*, StopContainer…), já documentados; só `GitOAuthStart` e `Unknown` ficam fora, com a razão no teste. Um `Command` novo sem entrada no manual quebra o CI.

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
- [ ] **Deploy antigo aparece `Stopped` enquanto o container ainda roda** (**a raiz que sobrou foi achada em 2026-11-07: o `reconcile` condenando o `Live` com o container ainda não religado — ver o item "Sem registro de deploy da api e do worker" acima, já corrigido; falta confirmar em produção.**) (visto no Chiquitos, 2026-11-04): ao entrar um deploy novo na fila/construção, o registro do
  deploy `Live` anterior vira `Stopped` em segundos (api 05:07:35, 14 s depois; worker 25 s depois), mas o container antigo só é trocado quando o novo
  fica pronto (até ~25 min depois) e o status do serviço segue `Running`. A GUI/webui mostram o serviço "parado" nesse intervalo. Não é o `stop.rs`
  (só roda no parar manual); achar o caminho que faz a transição e manter `Live` até a troca (ou usar `Pruning`, "substituído").
  **Investigado em 2026-11-05 (sem commit):** causa provável — `update_status(Queued|Deploying, None)` gravava `live_container_id = NULL` com o container antigo ainda no ar; sem o id, o `reconcile` (`api/handlers/deploys/reconcile.rs`) lê "container não está rodando" e vira o `Live` em `Stopped`. Corrigido: `Queued`/`Deploying` preservam o id (`store/services.rs`, com teste) e o `reconcile` reconfere o guard antes de condenar. **Não reproduzi o `Stopped` em 14 s num daemon real**; confirmar no Chiquitos num deploy longo (registro segue `Live` até a troca, `live_container_id` não-nulo durante o build).
