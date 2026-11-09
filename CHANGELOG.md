# Changelog

Histórico das últimas versões do Rustploy (`v0.1.25` a `v0.1.53`, de 28/10/2026 a 09/11/2026), do mais novo para o mais antigo.
Levantado a partir das tags do agregador, dos ponteiros dos submódulos, dos commits entre cada par de tags, das releases do GitHub
e do [`docs/release.md`](docs/release.md) (onde estão as notas de publicação, com o que foi e o que **não** foi verificado).

## Como ler este arquivo

- **A tag não é a versão dos programas.** A tag `vX.Y.Z` é do agregador; o daemon (`rustploy`), o app (`rustploy-gui`) e o
  `rustploy-shared` têm numeração própria, e o `cargo-deb` ainda acrescenta a revisão (`rustployd_0.1.35-1_amd64.deb`). A tabela abaixo
  liga cada tag às versões que ela carrega.
- **Pacotes assinados.** Cada `.deb` e o zip do Windows têm um `.minisig` (minisign, chave do dono). Uma release sem `.minisig` não é
  oferecida pelos updaters. Veja "Assinatura dos pacotes" em [`docs/release.md`](docs/release.md).
- **Release no GitHub × só tag.** Hoje só `v0.1.38` a `v0.1.43` aparecem na lista de releases do GitHub. As anteriores existem como
  **tag** no repositório; o `docs/release.md` registra que várias delas foram publicadas na época (por exemplo `v0.1.31` e `v0.1.35`).
- **`feat!`** marca mudança que quebra compatibilidade (protocolo, YAML ou schema). Quando há `!`, atualize daemon, app e web UI juntos.

## Resumo

| Tag | Data | Daemon (`rustploy`) | App (`rustploy-gui`) | `rustploy-shared` | Destaque |
|---|---|---|---|---|---|
| [v0.1.53](#v0153--2026-11-09) | 2026-11-09 | **0.1.45** | 0.1.48 | 0.23.0 | O daemon cifra no boot o token de webhook e o `client_secret` que a migração deixou em claro (destrava o sync com o git) |
| [v0.1.52](#v0152--2026-11-09) | 2026-11-09 | **0.1.44** | **0.1.48** | 0.23.0 | Corrige a importação de dados no Podman e os nomes na janela Reordenar serviços |
| [v0.1.51](#v0151--2026-11-09) | 2026-11-09 | **0.1.43** | **0.1.47** | **0.23.0** | Exportar e importar os dados de um serviço (aba Dados), com cifra `age` |
| [v0.1.50](#v0150--2026-11-08) | 2026-11-08 | **0.1.42** | **0.1.46** | **0.22.0** | Reordenar os serviços de um projeto |
| [v0.1.49](#v0149--2026-11-08) | 2026-11-08 | **0.1.41** | **0.1.45** | **0.21.0** | Deploy com dependências e Deploy do projeto, com modal da ordem |
| [v0.1.48](#v0148--2026-11-08) | 2026-11-08 | **0.1.40** | 0.1.44 | 0.20.0 | Chave mestra e de visitante; níveis do agente |
| [v0.1.47](#v0147--2026-11-07) | 2026-11-07 | **0.1.39** | 0.1.44 | 0.20.0 | A credencial do provider git só vai para o servidor do provider |
| [v0.1.46](#v0146--2026-11-07) | 2026-11-07 | **0.1.38** | 0.1.44 | 0.20.0 | Deploy `Live` não é mais condenado no boot do daemon |
| [v0.1.45](#v0145--2026-11-07) | 2026-11-07 | **0.1.37** | **0.1.44** | **0.20.0** | Edição de variável no lugar, push-to-deploy do registry, `EnvValueSet` |
| [v0.1.44](#v0144--2026-11-06) | 2026-11-06 | **0.1.36** | **0.1.43** | **0.19.0** | Logs por execução: lista, ver, excluir e limpar |
| [v0.1.43](#v0143--2026-11-06) | 2026-11-06 | 0.1.35 | **0.1.42** | 0.18.0 | Seletor de idioma no rodapé do login e em Configurações |
| [v0.1.42](#v0142--2026-11-06) | 2026-11-06 | **0.1.35** | 0.1.41 | **0.18.0** | `product` no lugar de `db_kind` (quebra), webhook form, IP fixo no alias |
| [v0.1.41](#v0141--2026-11-05) | 2026-11-05 | 0.1.34 | **0.1.41** | 0.17.0 | Fix da janela de logs do app |
| [v0.1.40](#v0140--2026-11-05) | 2026-11-05 | **0.1.34** | **0.1.40** | **0.17.0** | Baixar a infra e log de runtime persistido (quebra) |
| [v0.1.39](#v0139--2026-11-05) | 2026-11-05 | **0.1.33** | 0.1.39 | 0.16.0 | Webhook filtra por `watch_paths` |
| [v0.1.38](#v0138--2026-11-04) | 2026-11-04 | **0.1.32** | 0.1.39 | 0.16.0 | Podman + ufw: DNS dos serviços, saída e isolamento |
| [v0.1.37](#v0137--2026-11-03) | 2026-11-03 | 0.1.31 | 0.1.39 | 0.16.0 | Tag a descartar (bug de ordem de regras do ufw) |
| [v0.1.36](#v0136--2026-11-03) | 2026-11-03 | 0.1.30 | 0.1.39 | 0.16.0 | Tag a descartar (manual com receita rejeitada) |
| [v0.1.35](#v0135--2026-11-03) | 2026-11-03 | **0.1.29** | **0.1.39** | **0.16.0** | Credenciais cifradas, renomear variável/secret, Storage |
| [v0.1.34](#v0134--2026-11-03) | 2026-11-03 | 0.1.28 | 0.1.38 | 0.15.0 | Restaurar o servidor a partir do git; `git` no `Depends` |
| [v0.1.33](#v0133--2026-11-02) | 2026-11-02 | 0.1.27 | **0.1.38** | **0.15.0** | Secrets: mover entre escopos, ver valor; dependências na GUI |
| [v0.1.32](#v0132--2026-11-02) | 2026-11-02 | **0.1.22** | **0.1.36** | **0.14.0** | Infra em YAML, secrets age, sync do estado com o git |
| [v0.1.31](#v0131--2026-10-31) | 2026-10-31 | **0.1.20** | **0.1.35** | 0.10.0 | Interface em inglês (padrão) e pt-BR nos dois clientes |
| [v0.1.30](#v0130--2026-10-31) | 2026-10-31 | 0.1.19 | **0.1.33** | 0.10.0 | Rede pelo `http()` do glacier, upload do zip com progresso |
| [v0.1.29](#v0129--2026-10-30) | 2026-10-30 | 0.1.19 | **0.1.32** | 0.10.0 | Abas de Settings estáveis; projeto como link no Deploy Engine |
| [v0.1.28](#v0128--2026-10-29) | 2026-10-29 | 0.1.19 | **0.1.31** | 0.10.0 | Rótulo do link de tabela sem vazar da coluna |
| [v0.1.27](#v0127--2026-10-29) | 2026-10-29 | **0.1.19** | **0.1.30** | 0.10.0 | Validação de variáveis; editor `.env` com erros por linha |
| [v0.1.26](#v0126--2026-10-29) | 2026-10-29 | **0.1.18** | **0.1.29** | 0.10.0 | Links de serviço e projeto nas tabelas |
| [v0.1.25](#v0125--2026-10-28) | 2026-10-28 | **0.1.17** | 0.1.28 | 0.10.0 | Só o bump, para testar o aviso de atualização |
| [v0.1.24](#v0124--2026-10-28) | 2026-10-28 | 0.1.16 | **0.1.28** | 0.10.0 | Aviso de atualização do daemon na tela de login do app |

Em **negrito**, o que mudou em relação à tag anterior. As datas seguem o `docs/release.md` (dia local do dono) e, nas tags sem entrada
lá, a data do commit da tag; o horário de publicação no GitHub, em UTC, está na tabela seguinte.

## Releases publicadas no GitHub (arquivos)

| Release | Publicada (UTC) | Daemon `.deb` (amd64 e arm64) | App `.deb` (amd64) | Windows |
|---|---|---|---|---|
| v0.1.43 | 2026-11-06 07:39 | `rustployd_0.1.35-1` | `rustploy-gui_0.1.42-1` | `rustploy-gui-windows-v0.1.43.zip` |
| v0.1.42 | 2026-11-06 05:50 | `rustployd_0.1.35-1` | `rustploy-gui_0.1.41-1` | `rustploy-gui-windows-v0.1.42.zip` |
| v0.1.41 | 2026-11-06 01:33 | `rustployd_0.1.34-1` | `rustploy-gui_0.1.41-1` | `rustploy-gui-windows-v0.1.41.zip` |
| v0.1.40 | 2026-11-05 07:13 | `rustployd_0.1.34-1` | `rustploy-gui_0.1.40-1` | `rustploy-gui-windows-v0.1.40.zip` |
| v0.1.39 | 2026-11-05 01:14 | `rustployd_0.1.33-1` | `rustploy-gui_0.1.39-1` | `rustploy-gui-windows-v0.1.39.zip` |
| v0.1.38 | 2026-11-04 02:07 | `rustployd_0.1.32-1` | `rustploy-gui_0.1.39-1` | `rustploy-gui-windows-v0.1.38.zip` |

Cada release traz também o `checksums.sha256` e um `.minisig` para cada `.deb` e para o zip.

---

## v0.1.53 — 2026-11-09

Daemon **0.1.45**; app **0.1.48** e `rustploy-shared` **0.23.0** não mudaram. Servidores migrados do SQLite para o YAML podiam ter tokens de webhook e o `client_secret` de um provider Git em claro (a migração copiou os valores como estavam), e o sync com o git recusava ligar (`PlaintextFound`). No boot, o daemon agora cifra esses valores **no lugar, sem mudar o valor** (a URL de webhook cadastrada no provedor segue valendo). É idempotente e, se falhar, não impede o boot. Código em `store/infra/seal.rs`.

## v0.1.52 — 2026-11-09

Correção da `v0.1.51`, que **não** deve mais ser usada para importar dados. (Uma primeira tentativa de tag `v0.1.52` foi apagada antes de assinar; o nome foi reaproveitado, porque nunca houve release com ele.) Daemon **0.1.44** e app **0.1.48**; `rustploy-shared` **0.23.0** não mudou.

### Corrigido
- **App: os nomes dos serviços sumiam na janela Reordenar serviços.** O índice ("1.") usava `.env__key`, com `width: 400`, numa janela de 440 px: comia a linha e empurrava o nome para fora. Agora tem classe própria (`.reorder__index`, largura 32).
- **Importar dados de um volume falhava no Podman** com `tar: short read`: o arquivo era passado como stdin/stdout herdado do `podman --remote`. Agora a cópia é explícita (pipe), em volumes e bancos.
  Achado ao rodar, pela primeira vez, o teste de integração contra um Podman de verdade; a `v0.1.51` **não** deve ser usada para importar.
- **Postgres:** a importação recria o database antes de restaurar (antes, uma tabela que não estava no pacote sobrava, e o rollback não a desfazia). MySQL/MariaDB já recriavam; no MongoDB só as coleções do pacote são substituídas.

## v0.1.51 — 2026-11-09

Exportar e importar os dados de um serviço. Daemon **0.1.43**, app **0.1.47**, `rustploy-shared` **0.23.0**.

### Adicionado
- **Aba Dados** no serviço (app e web UI): leva os dados para um arquivo `.rpdata` e os traz de volta em outro serviço do mesmo tipo, no mesmo servidor ou em outro.
  Tudo parte de um `service_id`: o daemon descobre os volumes sozinho (etiqueta da stack Compose e `volumes` do spec; pastas do servidor só passando por uma lista de proibidas).
- **Bancos** (Postgres, MySQL, MariaDB, MongoDB): dump lógico feito dentro do container do serviço, com ele no ar, sem passar credencial pelo daemon. **Volumes comuns**: tar,
  com o serviço parado durante a cópia e religado depois.
- **Importar com rede de segurança:** o pacote é conferido (SHA-256, motor, versão) antes de tocar em qualquer volume; o conteúdo atual é guardado e devolvido se a troca falhar.
- **Cifra com `age`:** ligada por padrão quando há `recovery_recipient`; abre no servidor de origem ou num restaurado com a mesma identidade.
- Protocolo (`rustploy-shared` 0.23.0): `ServiceDataPlan { service_id }` (permitido ao observador). Os bytes vão por `GET /api/services/<id>/data/export` e
  `POST /api/services/<id>/data/import`, **só com a chave mestra**.

### Atenção
- Atualize **daemon e app juntos**: a aba Dados do app 0.1.47 chama um comando que só o daemon 0.1.43 conhece.

## v0.1.50 — 2026-11-08

Reordenar os serviços de um projeto. Daemon **0.1.42**, app **0.1.46**, `rustploy-shared` **0.22.0**.

### Adicionado
- **Botão "Reordenar serviços"** no cabeçalho do projeto (app e web UI): abre um modal com a lista de serviços para arrastar (como as variáveis de ambiente; na web UI também
  com ↑/↓) e os botões **Reordenar** e **Cancelar**. Ao confirmar, os cards se reordenam na hora.
- **A ordem fica no servidor**, no `project.yml` (campo `service_order`, lista de ids, omitido quando vazio), portanto vai para o git. Serviço novo entra no fim; serviço apagado
  sai da ordem sozinho. Projetos nunca reordenados mantêm o YAML de antes.
- Protocolo (`rustploy-shared` 0.22.0): `ProjectServicesReorder { project_id, service_ids }`; nível `operator` para o agente. Id de outro projeto, repetido ou projeto
  inexistente dá erro e não muda nada.

### Alterado
- O daemon deixou de ordenar todos os serviços por data de criação: lista projeto por projeto, na ordem de exibição (`service_order`, depois criação).

### Atenção
- Atualize **daemon, app e web UI juntos**: o app 0.1.46 chama um comando que só o daemon 0.1.42 conhece.

## v0.1.49 — 2026-11-08

Deploy com dependências e Deploy do projeto. Daemon **0.1.41**, app **0.1.45**, `rustploy-shared` **0.21.0**.

### Adicionado
- **O botão Deploy do serviço enfileira a árvore de dependências inteira**, na ordem (`depends_on` do mesmo projeto mais as arestas implícitas do acesso a banco
  compartilhado), em vez de só o serviço. Se uma dependência falha ou é parada, quem espera por ela é **pulado** na fila.
- **Modal com a ordem** no app e na web UI: ao clicar em Deploy num serviço com dependências, aparece a lista na ordem da fila e os botões **Confirmar**, **Só o serviço** e
  **Rejeitar**. Sem dependências o deploy sai direto, como antes. No app o modal do serviço é um diálogo de escolha (lista com as três ações); na web UI são três botões.
- **Botão Deploy do projeto** (cabeçalho do projeto, nos dois clientes): o modal mostra a ordem de tudo que será deployado, com **Aceitar** e **Rejeitar**.
- Protocolo (`rustploy-shared` 0.21.0): `DeployWithDependencies`, `ProjectDeploy` e os planos sem efeito `DeployWithDependenciesPlan` e `ProjectDeployPlan`; resposta
  `DeployBatch` (serviço, onda, estado e id do deploy de cada item). Níveis do agente: `operator` para os dois deploys e `observer` para os planos.

### Corrigido
- **Dependência impossível de resolver agora dá erro ao salvar**: serviço de outro projeto, inexistente ou ciclo, inclusive o formado pelo acesso a um banco compartilhado
  (`SharedAccessGrant` também recusa o ciclo). Nada é salvo.

### Atenção
- Atualize **daemon, app e web UI juntos**: o app 0.1.45 chama comandos que só o daemon 0.1.41 conhece (um daemon antigo recusa o plano e o app cai no deploy simples).

## v0.1.48 — 2026-11-08

Chave mestra e chave de visitante, com níveis. Só o daemon muda (**0.1.40**); o app (0.1.44) e o `rustploy-shared` (0.20.0) ficam como estão.

### Adicionado (segurança)
- **Chave mestra opcional** (`[api].master_token`). Até aqui o `/api/rpc` ignorava as proibições do agente que valiam só no `/mcp`, e as duas rotas
  usavam o mesmo token: quem o tinha (um agente com `curl`, por exemplo) fazia `SecretReveal` e lia as senhas em claro. Agora, **com a mestra
  configurada**, a chave atual (`[api].token`) vira a de **visitante**: ela passa pelas proibições, pelo nível, pela restauração de variáveis
  mascaradas e pela redação das respostas em **qualquer rota** (`/mcp` e `/api/rpc`). Só a mestra tem acesso total.
- `rustployd access-key --master` gera a mestra (256 bits; falha em vez de gerar uma chave fraca), grava `master_token` no `config.toml` e
  imprime o token de conexão dela; vale depois de reiniciar o daemon. **Sem mestra, nada muda.**
- **Níveis do visitante** (`[api].visitor_level`): `observer` (só lê), `operator` (padrão: deploy, parar/reiniciar, variáveis, secrets, domínios,
  réplicas, ligar/desligar jobs; **não** muda o que um serviço ou job roda — fonte, imagem, comando, volumes, compose) e `admin`. A tabela de níveis é
  um `match` exaustivo sobre os 127 comandos: um comando novo só compila depois de escolher o nível.
- Para o visitante: `GET /api/events` (traz variáveis em claro) e o upload de código de um serviço (abaixo de `admin`) são recusados; a mestra nunca é
  devolvida por `/api/access-key` nem aceita em `/api/login`.

### Corrigido
- **`JobUpdate` restaura as variáveis mascaradas**, como o `ServiceUpdate`: o agente via `DB_PASSWORD=***` num job e não conseguia editá-lo (nem
  ligá-lo ou desligá-lo) sem reenviar uma senha que nunca viu.
- **`InfraAuditAllow` é proibido ao agente**: marcar uma suspeita como "não é segredo" tira a trava que impede valor em claro de ir para o git.

### Atenção
- **Quem gerar a mestra precisa entrar de novo na GUI e na webui**, com o token de conexão **da mestra**; quem continuar com a chave antiga vê
  "proibido" nas ações que o nível dela não cobre. Perdeu a mestra? Edite `[api].master_token` no `config.toml` e reinicie.
- Um visitante `admin` ainda consegue, no limite, montar pastas do servidor num container próprio (ele cria serviços): trate `admin` como acesso total.
  O token de um **secret** usado em `credentials` continua sem host associado.

### Planejado, não feito
- `docs/plano-exportar-importar-dados-de-servico.md`: exportar e importar os **dados** de um serviço (dump lógico para banco, tar para volume, sempre por
  `service_id`); aguarda a revisão do dono.

### Publicação
- `rustploy` 0.1.40 no crates.io; release assinada.

---

## v0.1.47 — 2026-11-07

Só o daemon muda (**0.1.39**); o app (0.1.44) e o `rustploy-shared` (0.20.0) ficam como estão.

### Corrigido (segurança)
- **A credencial de um provider git só é enviada ao servidor do próprio provider.** Antes, o token de uma conta conectada (GitHub, Gitea) era
  embutido em **qualquer** URL `https://` informada no clone de um serviço, de um job ou no `remote` do backup do estado em git: bastava apontar
  para um servidor que respondesse "401, preciso de login" para ele receber o token. Agora o host (e a porta) da URL têm de ser os do provider
  (`github.com` para o GitHub público, o host da **BASE URL** nos demais); em outro servidor o clone segue **sem** a credencial e o `remote` do
  `state_git` é recusado já na configuração.
- **Atenção ao atualizar:** um provider cuja **BASE URL** difere do endereço real dos repositórios deixa de autenticar; corrija a **BASE URL**.
  O token de um **secret** usado em `credentials` não tem host associado e continua sendo enviado a qualquer URL (ver o plano abaixo).

### Interno
- O teste de cobertura do manual do agente passou a ter o `match` de nomes do `dispatch` **sem braço curinga**: 7 comandos (`DeployQueue*`,
  `LogsSubscribe`/`Unsubscribe`, `MetricsSubscribe`/`Unsubscribe`) ficavam de fora sem ninguém ver.

### Planejado, não feito
- `docs/plano-limites-do-agente-mcp.md`: as restrições do agente valem só no `/mcp`; o `/api/rpc` com o mesmo token as ignora (feito na `v0.1.48`).

### Publicação
- `rustploy` 0.1.39 no crates.io; release assinada.

---

## v0.1.46 — 2026-11-07

Só o daemon muda (**0.1.38**); o app (0.1.44) e o `rustploy-shared` (0.20.0) ficam como estão. `glacier-ui` **0.152.1**.

### Corrigido
- **Deploy `Live` condenado no boot do daemon.** O `reconcile` marcava o deploy `Live` como `Stopped` e **apagava o `live_container_id`** quando o container
  ainda não tinha sido religado pelo autostart; depois o container voltava e o deploy ficava `Stopped` para sempre (a aba Deployments mostrava o
  serviço "parado" e o histórico sem um `Live`). Agora o `Live` é poupado na janela do boot (intenção "no ar" e daemon com menos de 10 min), e o
  deploy mais recente que o próprio `reconcile` condenou volta a `Live` quando o container está rodando — o que também conserta o que já estava
  gravado. Um "Parar" feito por uma pessoa não é revertido.
- `glacier-ui` 0.152.1: `pedir_texto`, `escolher_cor`, o seletor de fonte e o diálogo de progresso não montavam o corpo desde a 0.149.0; 48 testes
  desatualizados do motor foram atualizados.

### Publicação
- `rustploy` 0.1.38 no crates.io; release assinada; `glacier-ui` 0.152.1.

---

## v0.1.45 — 2026-11-07

Edição de variável no lugar, push-to-deploy do registry embutido e `EnvValueSet`. Muda o daemon (**0.1.37**), o app (**0.1.44**) e o
`rustploy-shared` (**0.20.0**, aditivo). `glacier-ui` **0.151.0** e **0.152.0**.

### Adicionado
- **Editar variável no lugar** (app e web UI, no projeto e no serviço): o ✏️ transforma a chave em um campo na própria linha e o novo ✎ faz o
  mesmo com o valor; Enter grava, ✕ desiste. Antes o ✏️ levava a variável para um formulário no topo da tela, e numa lista grande a janela
  ficava longe dela. Numa variável que aponta para um secret, o campo mostra `secret:NOME`, nunca o valor.
- Protocolo: **`EnvValueSet`** troca só o valor de uma variável, na mesma posição (também disponível ao agente MCP pelo `rpc`).
- **Push-to-deploy do registry embutido**: `ServiceSpec.registry_auto_deploy` (opt-in, na aba Advanced) redeploya o serviço a cada
  `docker push` da mesma imagem e tag; novo `Event::RegistryPush`.
- App: o campo de texto **rola na horizontal** acompanhando o cursor (digitar, colar, setas); o **toast** sobe ao aparecer, tem uma barra de
  tempo lateral e desliza para fora; "Salvar secret" ao lado do valor; a aba Variáveis do projeto rola inteira e não corta mais o botão Adicionar.
- `glacier-ui` 0.152.0: `textinput(autofocus = true)`.

### Alterado
- `/api/rpc` **recusa** (400) um `ServiceUpdate` cujo payload omite campos que o serviço já tem preenchidos (o app e a web UI já devolvem o spec
  completo). O MCP segue só com o aviso do manual.
- Manual do agente: 13 comandos que faltavam (`StateGit*`, `UpdateStatus`/`ApplyUpdate`/`UpdateRollback`, `ServiceWarm*`, `StopContainer`…);
  um teste agora falha se um `Command` novo ficar sem entrada.

### Corrigido
- **Boot:** um serviço com `healthcheck: None` que não escuta porta (um worker de fila) era marcado como erro após 120s em todo boot e arrastava
  quem depende dele. Agora, com o container rodando, conta como pronto.
- A web UI não ficava mais presa ao CSS/JS antigo depois de uma atualização do daemon (o service worker buscava o app shell pelo cache HTTP imutável).
- `parse_mem` estourava o `u64` com um valor enorme num manifesto (`99999999999g`); a fila de deploy sobrevive a um mutex envenenado.

### Publicação
- `rustploy-shared` 0.20.0, `rustploy` 0.1.37 e `rustploy-gui` 0.1.44 no crates.io; release assinada; `glacier-ui` 0.152.0.

---

## v0.1.44 — 2026-11-06

Logs por execução. Muda o daemon (**0.1.36**), o app (**0.1.43**) e o `rustploy-shared` (**0.19.0**, aditivo).

### Adicionado
- Aba **Logs**: "Logs de execução" mostra só a execução atual; abaixo, a lista **Execuções anteriores** (intervalo, deploy, imagem, linhas,
  tamanho, desfecho) com **Ver logs** (janela com data e hora), **Excluir** e **Limpar selecionados**. Na webui e no app.
- Protocolo: `LogsExecutions`, `LogsExecutionDelete` e `container_ids` em `LogsHistory`.
- App: título do serviço editável (grava sozinho), opção nula nos seletores de mover e botões travados sem destino.

### Alterado
- O log de runtime fica em uma pasta por execução; a retenção apaga execuções inteiras. Na primeira subida o daemon migra os logs da v0.1.40.

### Publicação
- `rustploy-shared` 0.19.0, `rustploy` 0.1.36 e `rustploy-gui` 0.1.43 no crates.io; release assinada; `glacier-ui` 0.150.1.

---

## v0.1.43 — 2026-11-06

Seletor de idioma do app. Só o app muda: **`rustploy-gui` 0.1.42**. Daemon 0.1.35 e `rustploy-shared` 0.18.0 não mudam.

### Alterado
- Os botões **English / Português** saíram da barra do topo. Na tela de login ficam em um rodapé discreto, no canto inferior direito;
  dentro do app, numa aba nova **Idioma** em **Configurações**. A web UI não mudou.
- Docs (pt e en) das páginas "Idioma da interface" e "Entrar" atualizadas.

### Publicação
- `rustploy-gui` 0.1.42 no crates.io; release assinada; comando de uma linha do app testado num `debian:12` limpo.

### Verificado
- `cargo fmt`, `clippy -D warnings`, 40 testes; a tela foi aberta e testada pelo dono antes do publish.
- No Chiquitos (daemon 0.1.35 e publisher novo do chickie) os `missed heartbeats` do RabbitMQ sumiram: a `api` ficou 43 min conectada.

### Não verificado
- Reconexão do publisher depois de uma queda do broker em produção (só o teste automático cobre).
- O falso `Stopped` do primeiro deploy depois do restart do daemon (`container not running at read time`) não tem causa conhecida e não repetiu.
- `install.ps1` e a conferência da assinatura num Windows real; atualização do app pelo botão.

---

## v0.1.42 — 2026-11-06

`product` no lugar de `db_kind`, webhook com formulário e IP fixo no alias de rede. **Daemon 0.1.35**, **`rustploy-shared` 0.18.0**;
o app 0.1.41 já usava o `product`.

### Quebra de compatibilidade
- **`ServiceSpec.db_kind` virou `product`** (`rustploy-shared` 0.18.0). O campo guardava banco, broker e storage; o nome mentia. O daemon
  converte o YAML e o SQL antigos na leitura (infra **schema 2**, com backup `infra.bak-schema1`). O manifesto do pacote de serviço
  aceita `db:` como alias de `product:`. O `/api/rpc` recusa `ServiceCreate` e `ServiceUpdate` com `db_kind` preenchido (cliente antigo):
  atualize o app junto.

### Adicionado
- **Webhook aceita `application/x-www-form-urlencoded`** (`payload=`) e ignora o `ping` do GitHub.

### Corrigido
- **IP fixo ao reconectar o alias de rede** (`attach_network_alias`): antes o container voltava com outro IPv4 e as conexões abertas
  (RabbitMQ no Chiquitos) morriam por heartbeat 2–3 min depois do deploy. Se o engine recusar o IP fixo, cai para IP novo com um aviso.
  Validado em Podman rootful 5.7.0, com e sem um segundo container na rede; sem o conserto o teste falha.
- `update_status(Queued|Deploying)` preserva o `live_container_id` e o `reconcile` reconfere o guard de deploy em andamento.
- Template MinIO do catálogo usa `cgr.dev/chainguard/minio`.

### Documentação
- Manifesto do pacote documenta `product` (e `db:` como alias), em pt e en.

### Publicação
- `rustploy-shared` 0.18.0 e `rustploy` 0.1.35 no crates.io. O `rustploy-gui` 0.1.41 já estava publicado.

### Verificado
- `make lint` e `make test` (485 testes do daemon); site com 927 páginas e 0 links quebrados; updater do daemon pelo botão e conversão
  `db_kind` → `product` num servidor com dados reais, sem problemas (conferido pelo dono).

### Não verificado
- IP fixo no Chiquitos foi conferido depois, pelos logs do broker (ver `v0.1.43`).

---

## v0.1.41 — 2026-11-05

Fix da janela de logs do app. Só o app: **`rustploy-gui` 0.1.41**. Daemon 0.1.34 e shared 0.17.0 inalterados.

### Corrigido
- A janela de logs da v0.1.40 dava `attempt to call a nil value` ao abrir **Logs**: `format.history_rows` não estava exportado pela fachada
  `views/scripts/format.luau`.
- O separador de container do log passa a sair em pt-BR no catálogo pt-BR (aparecia em inglês).

### Verificado
- `cargo fmt`, `clippy -D warnings`, 40 testes e `cargo publish --dry-run`; a GUI abre a janela de logs da `api` do Chiquitos e mostra o
  histórico (11 linhas).

### Não verificado
- O botão **Carregar anteriores** (só aparece com mais de 500 linhas guardadas num serviço que não seja Compose).

### Armadilha
- Função nova em `format/*.luau` precisa entrar também na fachada `views/scripts/format.luau`; os testes Luau não pegam a falta.

---

## v0.1.40 — 2026-11-05

Baixar a infra e log de runtime persistido. **Daemon 0.1.34**, **app 0.1.40**, **`rustploy-shared` 0.17.0** (quebra de protocolo).

### Quebra de compatibilidade
- Saem a aba **Infra as Code** e os comandos `ManifestApply`, `ManifestExport`, `ManifestExportAll` e `ManifestImport`. GUI e web UI antigas
  contra este daemon perdem a aba IaC: atualize os dois juntos.

### Adicionado
- **Baixar a infra** (`InfraDownload`): devolve os arquivos que o `state_git` grava, com os secrets como `ENC[age:…]`; recusa com
  `PlaintextInInfra` se o git também recusaria. Botão na aba **Sync** da GUI e da web UI.
- **Log de runtime persistido** (`LogsHistory`): arquivos por serviço em `<dados>/logs/`, 14 dias e 50 MiB (`[logs]` no `config.toml`),
  capturados antes de remover o container (troca e rollback). A janela de logs semeia do histórico e ganha **Carregar anteriores**.
- Abrir **Settings** carrega a lista de contas Git sozinho (GUI e web UI).

### Corrigido
- Links de serviço e projeto voltam a clarear no hover.

### Alterado
- O daemon deixa de referenciar o `rustploy-shared` por `path`: usa o 0.17.0 do crates.io. `cargo install rustploy` volta a refletir a `main`.

### Publicação
- `rustploy-shared` 0.17.0, `rustploy` 0.1.34 e `rustploy-gui` 0.1.40 no crates.io.

### Verificado
- `make lint` e `make test` (476 testes do daemon), `glacier test` 399, web UI headless 16, site 927 páginas e 0 links quebrados, CI da tag,
  comando de uma linha num `debian:12` limpo (daemon e app, com a assinatura conferida), `InfraDownload` e o log persistido num daemon local.

### Não verificado
- A captura do container antigo na troca (`Promoting`) num engine real; a janela de logs da GUI nativa; o `install.ps1`.

### Armadilhas
- O Podman entrega timestamps de log com resolução de segundo: a deduplicação do `log_store` precisa de instantes únicos.
- `pkill -f` casa com a própria linha de comando do shell: mate por PID exato.

---

## v0.1.39 — 2026-11-05

Webhook filtra por `watch_paths`. **Daemon 0.1.33**; app 0.1.39 (formatação dos `.gv`); shared 0.16.0 não mudou.

### Adicionado
- O webhook de deploy (`POST /webhook/<id>/<token>`) respeita o `watch_paths` do serviço Git: com o campo preenchido, só dispara se algum
  arquivo do push (`commits[].added|modified|removed`) casar; senão responde `200 skipped: no watched path changed`.
- **Na dúvida faz o deploy** (nunca perde um push): corpo sem a lista, `commits` vazio, commit sem alguma das listas, merge sem arquivos ou
  10+ commits. Deploy manual e o do MCP ignoram o filtro. Aceita arquivo, diretório ou glob simples (`*`, `**`).
- Para que serve: várias aplicações na mesma branch (chickie deixa as branches `deploy/*`/`main-*` e o `make sync`).

### Documentação
- Manual do agente: bucket e migrations como passo de provisionamento em pré-deploy; o comando de gerar/rotacionar segredo ainda não existe.

### Verificado
- `make lint` e `make test` (462 testes do daemon, 10 do filtro) e um daemon local real com 8 payloads. Release conferida em `debian:12`
  (daemon 0.1.33 e app 0.1.39).

### Não verificado
- Com um GitHub de verdade (o formato do payload vem da documentação dos provedores); `install.ps1`; o daemon sob systemd.

---

## v0.1.38 — 2026-11-04

Podman + ufw: DNS dos serviços e saída dos containers. **Daemon 0.1.32**; app 0.1.39; shared 0.16.0.

### Corrigido
- Em servidor com Podman rootful e `ufw` (`deny incoming` e `deny routed`), os nomes `rp_*` não resolviam dentro dos containers e nada saía
  para a internet. Novo auxiliar `rustploy-podman-ufw` (em `/usr/libexec/`), chamado pelo `postinst` e pelo `install.sh --engine podman`:
  detecta as faixas das redes, libera DNS e saída e **isola as redes entre si** (o `deny` antes do `allow`).
- O auxiliar confere a **ordem** das regras de rota e refaz com o `deny` no topo (`ufw route insert 1`): com o `allow` já presente, um `deny`
  acrescentado depois ficava atrás e perdia o efeito.
- `ServiceReload` não relê secrets (só para e inicia o mesmo container): trocar um secret exige redeploy; o manual diz isso.

### Verificado
- Num servidor real (DNS por nome, saída e portas internas com o isolamento ativo) e num laboratório
  (`tests/e2e-podman-ufw/run.sh`: Podman rootless privilegiado, `ufw` 0.36 e `iptables-nft` reais).

### Observação
- As tags `v0.1.36` e `v0.1.37` ficaram no repositório porque apagar tag remota foi bloqueado. Vale a `v0.1.38`.

---

## v0.1.37 — 2026-11-03

**Não usar.** Daemon 0.1.31. Tirou do manual a "receita" de rotação de senha por serviço descartável, mas o auxiliar do ufw tinha um bug de
ordem de regras, achado depois pelo laboratório e corrigido na `v0.1.38`. A execução foi cancelada; descarte qualquer rascunho que apareça.

---

## v0.1.36 — 2026-11-03

**Não usar.** Daemon 0.1.30. Liberou o ufw para os containers do Podman (DNS por nome e saída, com detecção das faixas) e trouxe no manual uma
receita de rotação de senha por serviço descartável, **rejeitada pelo dono**. A execução foi cancelada; **não assine o rascunho**.

### Adicionado
- `feat(podman)`: libera o ufw para os containers; receita de bucket com `curl --aws-sigv4` (o `aws-cli` rejeita hostname com `_`).

---

## v0.1.35 — 2026-11-03

Credenciais cifradas por padrão, renomear variável/secret, Storage (RustFS e MinIO) e agente que nunca vê segredo. **Daemon 0.1.29**,
**`rustploy-shared` 0.16.0**, app 0.1.39.

### Adicionado
- **Identidade `age` no lugar da `master.key`** para cifrar os secrets.
- **Renomear** chave de variável (`EnvKeyRename`) e secret (`SecretRename`); senhas do wizard geradas pelo daemon.
- **Storage** no wizard (RustFS e MinIO) com credenciais geradas e cifradas; categoria `storage` na tela Em execução.
- **O agente (MCP) nunca recebe o valor de um secret** (Fase A).
- Web UI: instruções de permissões do token GitHub (PAT fine-grained). Campo de senha só onde existe (Kafka e NATS não têm).

### Corrigido
- Padrões de usuário e banco do wizard quando o chamador não preenche; o ✏️ da variável do serviço na linha normal, não na de arrastar;
  MinIO usa `cgr.dev/chainguard/minio`; `clippy::double_must_use` do `async_trait` (clippy 1.99).

### Observação
- Nesta release o daemon referenciava o `shared` por `path` e o `cargo install rustploy` entregava a 0.1.28. Resolvido na `v0.1.40`.
- `minio/minio` e `minio/mc` não são mais puxáveis (Docker Hub e Quay).

### Verificado
- `make lint`, `make test` por repo, `glacier test` (406), e2e do updater (11 cenários), site com 0 links quebrados; comando de uma linha
  pela URL pública em `debian:12` (daemon 0.1.29 e app 0.1.39); `install.ps1` até o SHA-256 num PowerShell no Linux.

---

## v0.1.34 — 2026-11-03

Restaurar o servidor a partir do git. **Daemon 0.1.28**; app 0.1.38; shared 0.15.0.

### Adicionado
- **Restaurar num servidor limpo a partir do git** (Fase 8 do sync de estado).
- **`git` no `Depends` do `.deb`** (Fase 9): o clone de serviços Git usa o `git` do servidor.

---

## v0.1.33 — 2026-11-02

Secrets entre escopos e dependências na GUI. **Daemons 0.1.23 a 0.1.27**, **app 0.1.38**, **`rustploy-shared` 0.15.0**.

### Adicionado
- **Secrets do serviço** na aba **Environment**: listar, salvar, apagar e mover variável para secret (GUI e web UI).
- **Mover variável entre serviço e projeto** (`SecretMoveOp::EnvServiceToProject` e `EnvProjectToService`), com 🔒 🔓 ↑ ↓ na GUI.
- **Ver o valor de um secret** (👁 🙈 📋), do projeto e do serviço, na GUI e na web UI.
- **Dependências e religar no boot** na aba **Advanced** do serviço (web UI e GUI).

### Alterado
- O daemon **deixa de criar a `master.key`**; só a lê se já existir.

### Corrigido
- `rustployd-fw.service` sobe em servidor sem `ufw` (`ReadWritePaths` com `-`); o log mostra a causa completa quando a identidade não cabe
  no `config.toml`.

---

## v0.1.32 — 2026-11-02

Infra em YAML como fonte da verdade, secrets `age`, dependências e sincronização do estado com o git. **Daemon 0.1.22**,
**app 0.1.36**, **`rustploy-shared` 0.14.0**.

### Adicionado
- **Infra em YAML como fonte da verdade**, com secrets `age`, dependências entre serviços e autostart (`ServiceSpec.depends_on`, shared 0.11.0).
- **Secrets com escopo de serviço**, `SecretMove`, `SecretReveal` e `InfraAudit` (shared 0.12.0).
- **Sincronização do estado com o git** (`StateGit*`, `InfraAuditAllow`; shared 0.13.0), com a Fase 7: fetch e pull validado, pausa por
  divergência, e itens removidos só marcam. Tela **Settings → Sync**, aviso no topo e containers órfãos com Parar/Remover (GUI e web UI).

### Alterado
- Templates `.gvb` viram `.gv` e o vocabulário passa a snake_case; `glacier-ui` 0.149.0.

---

## v0.1.31 — 2026-10-31

Interface em inglês (padrão) e pt-BR nos dois clientes. **App 0.1.35**, **daemon 0.1.20**; shared 0.10.0.

### Adicionado
- **Seletor de idioma** (English e Português) lembrado entre sessões, no app desktop e na web UI. Textos de tela passam a ser comparados em inglês nos testes.

### Alterado
- `width` e `spacing` dos `form(...)` vão para classes do `.gss`; `text("…")` sem `content =` em todo o código; o hover da aba ativa
  clareia o azul em vez de cair no cinza (app 0.1.34).

### Verificado
- Comando de uma linha pela URL pública em `debian:12`: app (`rustploy-gui 0.1.35-1`) e daemon (`rustployd 0.1.20-1`; sem systemd no
  contêiner, o serviço não sobe). A conferência da assinatura no Windows segue sem teste real.

### Armadilha
- `make test` caiu no link da GUI (`signal 7 [Bus error]`) com o disco a 92% enquanto rodavam outros builds: não é do código.

---

## v0.1.30 — 2026-10-31

Rede e upload do app. **`rustploy-gui` 0.1.33**; daemon 0.1.19; shared 0.10.0.

### Adicionado
- Rede pelo `http()` do `glacier-ui` 0.145; upload do zip com **progresso e Cancelar**; **atualizar o app em thread, sem `curl`**; reconexão
  do SSE; hover com `transition`.
- Projeto como link na fila e em "Executando" do Deploy Engine.

### Alterado
- Gradiente leve nos botões `primary`, `blue` e `danger` (`glacier-ui` 0.142).

---

## v0.1.29 — 2026-10-30

Abas de Settings estáveis. **`rustploy-gui` 0.1.32**; daemon 0.1.19; shared 0.10.0.

### Corrigido
- As abas de Settings não oscilam mais (`tabbar active_bold=false`, `glacier-ui` 0.140).

### Alterado
- O projeto vira link no histórico do Deploy Engine.

---

## v0.1.28 — 2026-10-29

**`rustploy-gui` 0.1.31**; daemon 0.1.19; shared 0.10.0.

### Corrigido
- O rótulo do `TableLink` usa 13px para não vazar da coluna.

---

## v0.1.27 — 2026-10-29

Validação de variáveis e editor `.env`. **Daemon 0.1.19**, **`rustploy-gui` 0.1.30**; shared 0.10.0.

### Adicionado
- **Validação de variáveis de ambiente** (nome e chave repetida) com aviso dos erros do editor `.env`.
- Editor `.env` da GUI com **erros por linha**, nome inválido e confirmação ao sobrescrever.
- Serviços do Deploy Engine viram links; as colunas truncam antes de vazar.

---

## v0.1.26 — 2026-10-29

Links nas tabelas. **Daemon 0.1.18**, **`rustploy-gui` 0.1.29**; shared 0.10.0.

### Adicionado
- Nomes de serviço e projeto viram links em Monitoring, Deployments e Ingress (web UI e GUI); selos sem colar e "Serviços no ar" no cabeçalho.

---

## v0.1.25 — 2026-10-28

**Daemon 0.1.17**; app 0.1.28; shared 0.10.0.

### Alterado
- Só o bump de versão, para testar o aviso de atualização na tela de login do app.

---

## v0.1.24 — 2026-10-28

**`rustploy-gui` 0.1.28**; daemon 0.1.16; shared 0.10.0.

### Adicionado
- Aviso de atualização do daemon na tela de login do app.

---

## Pendências conhecidas (até a v0.1.43)

- `install.ps1`: o atalho do Menu Iniciar (COM do Windows) e a assinatura com o `minisign.exe` (Windows PowerShell 5.1) não têm teste real.
- O `.deb` arm64 só foi compilado, nunca instalado numa máquina arm64.
- A migração Docker → Podman com dados reais foi testada só com stubs.
- O falso `Stopped` ("container not running at read time") visto uma vez no primeiro deploy depois do restart do daemon não tem causa
  conhecida; se repetir, o `journalctl -u rustployd` do momento é o que falta.
- Detalhes e armadilhas de cada publicação: [`docs/release.md`](docs/release.md).
