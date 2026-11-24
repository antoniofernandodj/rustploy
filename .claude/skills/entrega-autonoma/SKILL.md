---
name: entrega-autonoma
description: Constrói uma funcionalidade nova do Rustploy de ponta a ponta SEM supervisão do dono — plano em docs/, implementação no daemon e nos DOIS clientes (GUI e webui), testes de verdade (inclusive de atualização: a versão nova não pode quebrar servidores que já têm dados em produção), publish completo assinado, docs pt/en no ar, índice, agregador e instalação local do app e do daemon por comando. Use quando o dono disser /entrega-autonoma, "faz tudo sozinho", "não vou poder supervisionar", "do plano ao publish" ou pedir uma feature com autonomia total.
---

# Entrega autônoma

O dono vai estar fora. Invocar esta skill **é** a autorização para tudo abaixo, nesta entrega:
commitar e dar push em todos os repos, `cargo publish`, criar tag e release, **assinar com o
`minisign.pass`** (método do shim, ver memória `feedback_assinar_release_com_minisign_pass`),
fazer deploy das docs e atualizar o app e o daemon **desta máquina**. Não vale para servidores
remotos de produção (Chiquitos, Itemize): neles, só leitura pelo MCP, salvo se o pedido disser o contrário.

**Não pare para perguntar.** Decisão de desenho que for do dono: escolha a opção mais conservadora
(a que não apaga nada e mantém o que existe funcionando), registre a escolha e o porquê no plano, e
siga. Pare só se continuar for destruir dados, ou se uma verificação falhar e você não souber corrigir.

## A regra que não se negocia: a versão nova não pode quebrar a anterior

O Rustploy **já roda em produção com dados de verdade** (Chiquitos, Itemize: bancos, S3, filas,
domínios com TLS). O dono atualiza esses servidores pelo botão ou pelo `install.sh`, sem ler o código.
Toda entrega tem de ser uma **atualização segura**: o servidor que estava na versão anterior sobe na
nova e continua servindo **os mesmos dados, nos mesmos domínios, sem nenhuma ação manual**. Uma
feature nova que exige "recriar o serviço", "redeployar tudo" ou "mexer no `config.toml`" para não
perder nada **não está pronta**.

O que isso quer dizer na prática:

- **Nada que localiza dados muda de nome.** Nome da stack Compose (`compose_project`), da rede do
  projeto (`network_name`), dos volumes, dos containers e dos ids ficam gravados e são reaproveitados.
  Mudar a fórmula vale só para o que for criado depois.
- **Um campo gravado não muda de significado em silêncio.** Se o código passa a ler um campo que já
  existe de outro jeito (exemplo real, v0.1.95: `VolumeMount.host_path` sem `/` deixou de ser pasta
  relativa e virou volume nomeado), ou mantenha o comportamento antigo para o que já existe, ou migre
  no boot, ou **prove que nenhum servidor usa aquele caso** (próximo item).
- **Antes de mudar um comportamento existente, confira o que a produção usa.** Pelo MCP, só leitura
  (`ServiceList` de cada projeto, `IngressRoutes`, `DockerVolumes`…), nos servidores registrados
  (`rustploy-chiquitos`, `rustploy-itemize`, `rustploy`). Escreva no plano o que foi conferido e onde.
- **Padrão novo = comportamento antigo.** Opção nova nasce desligada ou com o valor que reproduz o que
  já acontecia (exemplo: `ServiceDelete.delete_volumes` é `false`: apagar serviço continua mantendo os
  dados).
- **Formato só cresce.** Campo novo com `#[serde(default)]` (e `skip_serializing_if` no YAML de
  infra); tabela/coluna nova no SQLite pelo `CREATE TABLE IF NOT EXISTS`/`add_column_if_missing`;
  mudança de formato de verdade só com migração no boot que não perde nada. Comando ou campo do
  protocolo não é removido nem renomeado: o antigo fica (marcado como legado) para clientes e agentes
  que ainda o usam.
- **Os dois clientes da versão anterior continuam funcionando** contra o daemon novo (o app desktop
  instalado pode demorar a ser atualizado): resposta nova só acrescenta campos.
- **Teste de atualização, com dados**, além dos testes da feature: daemon isolado na **versão
  publicada anterior** (o `.deb` ou o binário da tag), com um projeto que tenha o que a produção tem
  (serviço Application com domínio, stack Compose com volume e dados gravados, secret, job), parado;
  o binário **novo** sobe no mesmo `db_path`; confira que os serviços, os volumes (o arquivo gravado
  continua lá), os domínios e um redeploy funcionam. Se a feature mexe no proxy, nos volumes, na rede ou
  no nome de algo, este teste é obrigatório; nos outros casos, registre por que não se aplica.
- **Voltar de versão:** diga no plano e nas notas se a versão anterior consegue ler o estado depois que
  a feature nova for usada (o YAML regravado por um daemon velho perde campos que ele não conhece).
- **Na dúvida, não quebre:** escolha o caminho em que nada do que já existe muda, mesmo que a feature
  fique menor, e registre o que ficou para depois.

## O pipeline, na ordem

1. **Estado limpo.** `git status` e `git fetch` nos dois sentidos em todos os repos (agregador,
   submódulos e `~/Development/rust/glacier-ui` se for mexer nele). `df -h /`: abaixo de ~15 GB livres,
   `cargo clean` no que não vai ser compilado (o disco já encheu no meio de um publish).
2. **Plano** em `docs/plano-<assunto>.md`, em linguagem simples, com contexto e porquês (o dono não
   gosta de plano condensado): o problema, o modelo, a tela nos dois clientes, a migração de dados,
   as fases, o que entra **nesta** entrega e o que fica para depois. Cabeçalho com o status. Uma seção
   **"Atualizar um servidor em produção"**: o que muda para quem já tem dados, o que foi conferido nos
   servidores pelo MCP e por que a atualização é segura (ver a regra acima).
3. **Implementação.** Leia antes o `AGENTS.md` (Convenções) e o `docs/indice/INDEX.md`.
   - Daemon (protocolo, handler, store) **e** GUI (`.gv`/`.gss`/Luau) **e** webui (`webui/`): toda
     feature de interface entra nos dois clientes.
   - **Toda ação destrutiva pede confirmação**, nos dois clientes (apagar, remover, esquecer, parar,
     limpar, revogar, sobrescrever). Ação nova na GUI entra na lista `busy_actions`.
   - **Modelo de dados:** campo novo com `#[serde(default)]`; mudança de formato no YAML de infra ou no
     SQLite exige migração no boot (`store/infra/migrate.rs`, migrações do sqlx) que não perde nada e é
     testada com um estado antigo de verdade. Diga no plano se a versão anterior consegue ler o novo.
4. **Testes que provam.** `make lint` e `make test` verdes; teste unitário do caso novo; quando a
   mudança for de comportamento, teste de ponta a ponta num **daemon de teste isolado** (memória
   `rustployd_sem_cli_sobe_o_daemon`) e, para tela, a GUI num Xvfb (memória
   `rustploy_gui_visual_smoke_xvfb`) com captura. Rode o mesmo roteiro no código antigo quando der,
   para provar que o teste pega o defeito. `glacier test` na GUI. Mexeu no updater: os dois `e2e-updater`.
   **Teste de atualização** a partir da versão publicada anterior, com dados, quando a regra acima pedir.
5. **Publish completo**: a seção "Publish" do `AGENTS.md` e o `docs/release.md`, sem pular nada
   (bump só do que mudou, commit e push antes do `cargo publish`, CI sem tag verde antes da tag,
   assinatura, conferência da release e do `install.sh` num `debian:12`, `install.ps1` até o SHA-256).
6. **Docs** pt e en no `rustploy-docs` (capturas com host e segredos borrados), `npm run build`,
   `check-links` com 0 quebrados, commit, push, `npx wrangler deploy` e `curl` nas páginas novas.
7. **Índice e agregador**: `make index`, commit dos ponteiros e do `docs/indice/`, push.
8. **Instalar nesta máquina, por comando** (sem pedir senha), depois que a release estiver publicada:
   - daemon local: ferramenta MCP `rustploy-localhost` → `rpc` com `ApplyUpdate`; espere o daemon
     voltar e confira a versão com `status`;
   - app desktop: `rpc` com `ApplyAppUpdate` no mesmo daemon local (o helper root dele instala o
     `rustploy-gui` assinado); confira com `dpkg -s rustploy-gui`. O app aberto continua na versão
     antiga até ser reaberto: avise.
   - Se o daemon recusar (`forbidden`, `allow_agent_apply` desligado), **não** ligue a opção nem procure
     outro caminho: é uma trava do dono. Registre e avise no relatório.
9. **Notas** da versão no `docs/release.md` (o que foi verificado e o que **não** foi, e se atualizar um
   servidor com dados é seguro e por quê), commit e push.
10. **Relatório final** ao dono, curto: o que entrou, o que foi verificado de verdade, o que não deu
    para verificar, qualquer decisão tomada no lugar dele e **se ele pode atualizar a produção sem
    medo** (com a evidência: o que foi conferido nos servidores e o teste de atualização).
