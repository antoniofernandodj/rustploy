# TODO

Lista corrida de pendências — coisas identificadas mas não feitas ainda,
de qualquer assunto. Sem cabeçalho de status por item (isso é para
`docs/plano-*.md`); aqui é só riscar/apagar quando resolver.

Ordenada por valor (o mais valioso primeiro). Ao acrescentar um item, encaixe-o na posição certa em vez de
colar no fim.

**Revisão de 2026-11-25:** os itens já feitos saíram, junto com as verificações "falta ver em produção" que os
servidores provaram na prática desde então (Chiquitos, Itemize e Itemize dev rodam o daemon 0.1.82 com tudo no ar).
Os planos concluídos foram para `docs/arquivo/`; o que eles ainda tinham por fazer e fazia sentido entrou aqui.
Em `docs/` ficaram só os guias de referência e os planos que ainda não terminaram.

- [ ] **Recuperação de servidor: a chave errada inutiliza o servidor** (e os outros cinco achados do teste de
  2026-11-13). Uma tentativa com a chave errada deixa um commit no repositório local e toda tentativa seguinte falha,
  inclusive com a chave certa; a mensagem culpa o repositório. Fazer os itens 2, 3 e 4 juntos (conferir a chave antes
  de tocar em qualquer coisa, desfazer o que a tentativa gravou, levar o motivo para a tela), depois as contas Git
  (item 1), segurar o backup automático de serviço cuja restauração falhou (item 5, falta a decisão do dono) e o aviso
  do RabbitMQ sem `hostname` (item 6). Plano: `docs/plano-ajustes-da-recuperacao.md`.

- [ ] **Endurecimento do daemon (estudo de 2026-11-10, nada aplicado de propósito).** Na ordem do estudo: S-1 (token do
  git fora do log e do argv no clone de deploy; o `state_git` já passa por variável de ambiente), S-3 + S-4 (timeouts,
  teto de conexões e de corpo na API e no ingress), S-9 (hardening do systemd), S-8 + P-5 (limites de container,
  rotação de log). Medir antes (seção "O que medir"). `docs/estudo-seguranca-e-desempenho.md`.

- [ ] **Chave S3 própria por app** no Storage (hoje a app usaria a chave root) e **rotacionar a senha do `db` do
  Chiquitos** (foi exibida a um agente antes da Fase A). Contexto: `docs/arquivo/plano-credenciais-cifradas-e-storage.md`.

- [ ] **Geração e rotação de segredos.** O `SecretGenerate` existe (G1 em parte). Faltam o `SecretRotate`, os botões
  "Gerar"/"Rotacionar" nos dois clientes, os tipos `Token`/`Uuid`/`Symbols`, o par de chaves VAPID/Ed25519 (G2), as
  receitas de rotação por motor (G3) e o `generate:` do manifesto (G4); faltam também respostas do dono (seções 2 e 3).
  `docs/plano-geracao-e-rotacao-de-segredos.md`.

- [ ] **Vínculos, Fase 3: criar o database gerenciado e um usuário próprio a partir do vínculo** (hoje o database nasce
  na tela do banco e o dono é o usuário do provedor). `docs/plano-recursos-e-vinculos.md`.

- [ ] **Chave de visitante: o que a pessoa vê.** Os níveis (`observer`/`operator`/`admin`) valem no `/mcp` e no
  `/api/rpc`, mas a GUI e a webui mostram a recusa como erro de comando: mostrar o nível da sessão e explicar o 403.
  Exercitar uma vez num servidor real o fluxo inteiro (`rustployd access-key --master`, reiniciar, entrar com a mestra,
  agente com a antiga). Decidir o `credentials` (secret) sem host associado. Contexto:
  `docs/arquivo/plano-limites-do-agente-mcp.md` §12.

- [ ] **Descrição: levar para projetos e jobs**, mostrar "editada em <data>" (para desconfiar de texto velho) e, no
  importador do Dokploy, trazer a descrição das aplicações (hoje só a dos projetos). Metadados chave/valor só se alguma
  tela precisar filtrar por eles. `docs/arquivo/plano-descricao-do-servico.md`.

- [ ] **`JobRunCancel` também cancela um pré-deploy**: registrar o `job_run` do pré-deploy em `active_jobs` (hoje o
  caminho é abortar o deploy). Contexto: `docs/arquivo/plano-atritos-do-agente.md`.

- [ ] **Backup em git: o que sobrou.** (1) Testes manuais do R10.7 3c: rollback real de Compose (`git revert` da
  remoção reata o container), divergência com GitHub/Gitea de verdade e o procedimento manual por SSH. (2) Comando para
  **recifrar os secrets antigos para a chave de recuperação** (os salvos antes de configurá-la, como o `REDIS_PASSWORD`
  do Chiquitos, só abrem com o `age.key` do servidor). (3) Ajustes visuais: `<select>` da webui com fundo branco,
  "Cancelar" em português nos diálogos da GUI em inglês (embutido no glacier), desalinhamento das linhas de env da webui
  com o botão 🔒. Contexto: `docs/arquivo/plano-backup-estado-em-git.md`.

- [ ] **Convergência de templates GUI↔webui, Fases 2–4** (Fases 0 e 1 feitas). Exigem alinhar o escopo com o dono antes.
  `docs/plano-convergencia-templates-gui-webui.md` e `docs/inventario-paridade-telas.md`.

- [ ] **i18n (GUI e webui): o que ficou de fora da primeira versão.** (1) Texto que o código grava em estado (status,
  toast, `ctx.*_message`) fica no idioma em que nasceu; (2) rótulos da bandeja seguem em inglês; (3) erros do daemon não
  são traduzidos; (4) sem plural nem formato de número/data por idioma na webui; (5) mensagens montadas por
  concatenação exigem a mesma ordem de palavras: trocar por mensagens inteiras com `{n}`.

- [ ] **Capturas em inglês para o site en**: 46 imagens das páginas en ainda usam a GUI em português. Lista em
  `docs/plano-capturas-en.md` (apague o arquivo quando zerar).

- [ ] **Chickie** (repositório do dono): levar para a `main` o conserto do `RabbitMQTaskPublisher::publicar` (descarta
  o canal morto e reconecta) e o que ficou nas branches `deploy/api`, `deploy/worker` e `deploy/scheduler` (URLs por
  partes, cache `.sqlx`, job de migrations, filas duráveis do RabbitMQ 4), quando o dono decidir. Preencher no servidor:
  SMTP, Asaas, VAPID, `JWT_SECRET`, `OWNER_EMAIL`, `FRONTEND_BASE_URL`.

- [ ] **Testar em máquinas de verdade o que só rodou em container.** (1) `install.ps1` num Windows real (atalho do Menu
  Iniciar, aviso do SmartScreen); (2) o `.deb` arm64 numa VPS arm64; (3) `install.sh --engine docker` e
  `--engine podman` de ponta a ponta numa VPS limpa. Detalhes em `docs/release.md`, "Ainda sem teste real".

- [ ] **Assinar o `.exe` do Windows** (SignPath, tier OSS): o job `sign-gui-windows` é pulado enquanto a variável de
  repositório `SIGNPATH_ORGANIZATION_ID` não existe. Passos em `docs/windows-code-signing.md`.

- [ ] **Resto do site de docs.** A captura de instalar a web UI como PWA (tirada à mão: a barra do navegador não sai na
  extensão do Chrome), o CI de qualidade do site (links, exemplos executáveis, Lighthouse) e o domínio definitivo
  (canonical, sitemap e `Sitemap:` do `public/robots.txt`). Contexto: `docs/arquivo/plano-site-documentacao.md`.

- [ ] **Remover o `vendor/iced_tiny_skia` quando o iced 0.15 sair.** O `text_editor` (`<textarea>`) vazava a linha
  parcial da borda no renderer de software; o upstream corrigiu na `master` (commit `23170119b`, 2026-01-28) mas nenhuma
  release 0.14.x tem. Quando houver: (1) glacier-ui migra `iced` e publica; (2) rustploy sobe o `glacier-ui`; (3) no
  `rustploy-gui`, apagar `vendor/iced_tiny_skia/`, `vendor/README.md` e o `[patch.crates-io]`; (4) conferir o recorte na
  janela de logs. Contexto em `vendor/README.md`.

- [ ] **Migração Podman -> Docker.** Hoje só existe Docker -> Podman; voltar é manual (o `install.sh` recusa). Fazer o
  caminho inverso com as mesmas garantias: backup, `podman save | docker load` antes de parar nada, cópia de todos os
  volumes, redes, parar o `rustployd` e os `rp_*`, rollback se falhar, trocar o `socket_path`, remover os drop-ins do
  Podman, reiniciar e redeployar o que estava no ar. O `migrate_to_podman` do `install.sh` serve de base.

- [ ] **Tirar o glacier-ui de dentro do iced (proposta, nada feito).** (A) o glacier-ui manda no layout (Taffy) e nos
  eventos e o iced vira só backend de desenho e entrada; (B) depois, spikes de outros backends atrás do mesmo
  `trait Backend`. `docs/plano-evolucao-do-backend-grafico.md`.

- [ ] **Página "Para quem vai contribuir" no site de docs.** Adiada de propósito: o projeto continua fechado a
  contribuições (decisão de 2026-10-16). Quando retomar, a página só condensa a Parte 3 do `AGENTS.md` e aponta para ele.
