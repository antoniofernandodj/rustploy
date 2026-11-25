# Plano: quatro abstrações novas — Release, Ambiente, Rota e Volume

> **Arquivado em 2026-11-25** (revisão das docs): concluído ou substituído. O que ainda vale fazer foi para o
> `TODO.md`; o que não fazia mais sentido saiu daqui.

> **Status (2026-11-24):** as quatro abstrações implementadas. **1. Release** publicada na v0.1.91.
> **2. Ambiente, 3. Rota e 4. Volume** (Fase 1) publicadas juntas na v0.1.94. **4. Volume, Fase 2**
> (volume nomeado em serviço Application, montar/adotar órfão, lista atualizada no fim do deploy) na
> **v0.1.95** (daemon 0.1.79, app 0.1.86): §4.6. As decisões que tomei sem o dono estão marcadas com
> **[decisão minha]** e reunidas no §7.

## 0. Por que estas quatro, e por que nesta ordem

O Rustploy já tem muitas abstrações por cima dos containers: projeto, serviço, secret, job, ingress,
dependência, vínculo, backup. Todas elas descrevem **o que você quer** que exista. Faltam quatro
coisas que hoje existem só **espalhadas em campos** ou nem existem:

| # | Abstração | O que ela nomeia | Por que agora |
|---|---|---|---|
| 1 | **Release** | "o que rodou": imagem + configuração congeladas de um deploy | o rollback de hoje **não existe de verdade** (§1.1) |
| 2 | **Ambiente** | "o mesmo projeto, outra instância" (homologação, produção) | precisa da Release para **promover** código de um para o outro |
| 3 | **Rota** | "o que o proxy faz com um domínio", fora do serviço | redirect, path, senha e troca de domínio entre serviços |
| 4 | **Volume** | "os dados", com dono e estado de backup | apagar um serviço deixa os volumes **órfãos e invisíveis** (§4.1) |

A ordem segue a dependência (o Ambiente usa a Release) e o custo: a Release é a mais barata e
conserta um defeito; o Ambiente é o mais caro, mas fica mais caro quanto mais tarde entra; Rota e
Volume são independentes.

---

## 1. Release — o deploy congelado

### 1.1 O problema, em termos simples

Hoje um deploy (`Deployment`, em `domain/deployment.rs`) guarda só: id, serviço, **nome da imagem**,
estado e horários. Isso tem três consequências:

1. **O rollback é de mentira.** O `Command::DeployRollback` só *procura* o deploy Live anterior e o
   devolve; não redeploya nada (`api/handlers/deploys/deploy_rollback.rs`, "só consulta"). O botão
   saiu das duas telas por isso (o da webui está comentado em `app.js`). O manual do agente ainda o
   cita como se fosse um rollback.
2. **A configuração do deploy se perde.** Se o que quebrou foi uma variável, uma porta ou o comando,
   não há registro de "com que configuração estava rodando quando funcionava". Só a imagem.
3. **A imagem nem sempre volta.** Em fonte *Registry* a imagem é uma tag (`app:latest`): depois de
   outro push, "a imagem anterior" é outra coisa. Em fonte *Git/Zip* a tag é `rp_<nome>:<id8>` por
   deploy, mas a limpeza agendada do Docker pode apagá-la.

### 1.2 O modelo

Uma **Release** é criada quando um deploy chega a **Live**. Ela guarda:

- `id` (`rel_…`), `service_id`, `deployment_id`, `number` (1, 2, 3… por serviço; o que a tela mostra
  como **v12**) e `created_at`;
- `image`: o nome usado no deploy, e `image_ref`: a referência **fixa** — o `RepoDigest`
  (`app@sha256:…`) quando a imagem veio de registry, ou o id da imagem (`sha256:…`) quando foi
  construída aqui;
- `spec`: uma cópia do `ServiceSpec` daquele momento (fonte, portas, env do serviço, comando,
  réplicas, healthcheck, domínios, vínculos…);
- `project_env`: uma cópia das variáveis do **projeto** daquele momento.

**[decisão minha] O valor dos secrets não é copiado.** As variáveis que são `Secret(nome)` ficam na
cópia como *referência*, do mesmo jeito que já estão no spec. Copiar o valor decifrado para o SQLite
espalharia segredo em claro, e cifrar uma segunda cópia duplicaria o trabalho da rotação de chaves.
Consequência, escrita na tela: **voltar para uma release usa o valor atual de cada secret**. Para
segredo, isso é quase sempre o que se quer (ninguém quer voltar para uma senha revogada).

Mora no **SQLite** (tabela `release`), junto com `deployment` e `build_log`: é histórico de
execução, não estado declarado. **Não entra no YAML de infra nem no git.**

### 1.3 Voltar para uma release (o rollback de verdade)

`Command::ReleaseRollback { release_id }` faz, em ordem:

1. Confere que a imagem ainda existe (local, ou que o registry ainda tem aquele digest). Se não
   existir, recusa com uma causa clara ("a imagem da v9 foi apagada pela limpeza de 2026-11-20").
2. **Grava o `spec` da release como o spec atual do serviço** (passa pela mesma validação de um
   `ServiceUpdate`). Isso é uma sobrescrita, então **pede confirmação** nos dois clientes. A versão
   anterior do spec não se perde: o estado de infra é versionado em git.
3. Dispara um deploy que **pula o build**: usa `image_ref` direto (estado `PullingImage` quando é de
   registry, `Staging` quando a imagem é local).

**[decisão minha] As variáveis do projeto não são restauradas**, só mostradas no diff. Elas são
compartilhadas por todos os serviços do projeto: voltar a env do projeto para consertar um serviço
quebraria os outros. A tela mostra "estas variáveis do projeto mudaram desde a v9" para a pessoa
decidir.

**Compose:** a release guarda o conteúdo do compose. Voltar refaz o `compose up` com aquele conteúdo.
As imagens dentro do compose não são fixadas por digest nesta entrega (fica no §1.7).

O `DeployRollback` antigo continua existindo, só consultando, para não quebrar quem o chama; o
manual do agente passa a apontar o `ReleaseRollback`.

### 1.4 Proteger as imagens das releases

**[decisão minha]** As imagens das **3 releases mais recentes** de cada serviço ficam protegidas.

*Mudou na implementação:* o plano dizia "uma tag extra protege", mas a limpeza com "todas as imagens"
(`docker image prune -a`) apaga **qualquer** imagem sem container, com tag ou sem. E o `image prune` não
aceita exceção por imagem. Então, havendo releases para proteger, a limpeza de imagens passa a escolher
e remover uma a uma (`features/maintenance/prune.rs::images_to_remove`): as sem container (nem parado),
as sem tag quando não é "todas", as mais velhas que o corte de idade, **menos** as das releases
protegidas (por id ou digest). Sem nada para proteger, continua o `image prune` de antes.

Releases mais antigas continuam na lista com o selo "imagem apagada" quando a limpeza levar a imagem:
o diff ainda funciona, só o "voltar" fica desabilitado. O número 3 é a constante
`PROTECTED_RELEASES_PER_SERVICE`; vira configuração se alguém pedir.

### 1.5 Diff entre releases

`Command::ReleaseDiff { from, to }` devolve, campo a campo, o que mudou: imagem, cada variável do
serviço (nome e se o valor mudou; **valor só quando não é secret**), variáveis do projeto, portas,
comando, réplicas, domínios, vínculos. É o "o que mudou entre a v11 e a v12?", útil para gente e
para o agente pelo MCP.

### 1.6 A tela, nos dois clientes

Na aba **Deployments** do serviço (GUI `service/deployments.component.gv`, webui no detalhe do
serviço), cada deploy que virou release ganha o selo **v12**, e a linha da release Live atual fica
marcada como **atual**. As ações ficam numa linha logo abaixo da linha do deploy (na linha principal
não cabiam: cortavam o ✕ na largura da coluna). Na GUI o "Comparar" é um painel na própria aba, que
ocupa o lugar da lista enquanto está aberto; na webui, uma janela. Duas ações por linha de release:

- **Comparar com a atual** → abre o diff (janela na GUI, modal na webui);
- **Voltar para esta** → confirmação mostrando o diff resumido e o aviso dos secrets → `ReleaseRollback`.
  Entra na `busy_actions`.

Não crio uma aba nova: "release" é o deploy que deu certo, e separar em duas listas confundiria.

### 1.7 Fases

- **Fase 1 (esta entrega):** tabela `release`, criação no Live, digest/id da imagem, tag de proteção,
  `ReleaseList`, `ReleaseDiff`, `ReleaseRollback`, tela nos dois clientes, manual do agente, docs.
- **Depois:** fixar por digest as imagens de dentro do compose; retenção configurável; "voltar" também
  as variáveis do projeto, com confirmação própria.

### 1.8 Migração e compatibilidade

Só uma tabela nova no SQLite (migração aditiva no `store/mod.rs`). Deploys antigos não viram release
retroativamente (não temos o spec deles). Uma versão anterior do daemon ignora a tabela.

### 1.9 O que foi verificado (2026-11-23)

- Testes unitários: diff (campo, variável do serviço e do projeto, secret como `secret:NOME`, comentário
  ignorado), referência fixa da imagem, separar `repo:tag`, numeração por serviço e idempotência,
  imagens protegidas por serviço e a escolha da limpeza. `make lint`, `make test`, `glacier test` (489).
- De ponta a ponta, num daemon isolado com Podman: zip v1 (MODE=a) → zip v2 (MODE=b) → diff → voltar
  para a v1: deploy **sem build**, container servindo "v1", spec com MODE=a, v3 "volta da v1" com a
  mesma imagem; apagar o serviço apaga as releases. Pela GUI (Xvfb): selos, Comparar, confirmação e o
  voltar (v4 "volta da v2"). Webui no Chrome headless: selos, botões e a janela Comparar.
- **Não verificado:** a limpeza de imagens de verdade (rodá-la no Podman desta máquina apagaria imagens
  do dono; só o teste unitário cobre a escolha); voltar uma imagem de registry baixando pelo digest;
  Compose.
- **Achado do ambiente:** nesta máquina o AppArmor nega ao serviço de API do Podman o sinal ao `pasta`
  ("rootless netns: kill network process: permission denied") quando o último container sai de uma rede
  bridge, e o deploy de Application cai no SwappingIn. Não é do rustploy; no teste, um container parado
  numa rede à parte mantém a netns viva.

---

## 2. Ambiente — o mesmo projeto, outra instância

### 2.1 O problema

Para ter homologação hoje, você cria outro projeto e recria os serviços à mão. Os dois não sabem que
um é cópia do outro: não dá para ver o que difere, nem levar "o código que passou em homologação"
para produção sem refazer o build (e um build novo pode sair diferente).

### 2.2 O modelo

**[decisão minha] Um ambiente é um projeto com um vínculo para o projeto de origem.** Isso reaproveita
tudo o que o projeto já isola: rede própria, nomes de container próprios (`rp_<id8>_…`), env própria,
jobs. A alternativa (um nível novo dentro do projeto) mexeria nos nomes de container e de rede de
todos os serviços existentes, que é exatamente o tipo de migração que derrubou coisa antes.

- `Project.environment: Option<EnvironmentOf { origin_project_id, name, service_origins }>` (ex.:
  `name = "homologação"`).

*Mudou na implementação:* o plano punha `origin_service_id` no `ServiceSpec`, mas o `ServiceSpec` é
construído à mão em ~40 lugares (testes, wizard, importador) e o campo só faz sentido num ambiente.
O irmão de cada serviço mora no próprio vínculo do projeto: `service_origins` (serviço do ambiente →
serviço da origem). Um serviço criado depois, só num lado, simplesmente não tem irmão.

### 2.3 O que dá para fazer

- **Criar ambiente** a partir de um projeto: copia os serviços, os jobs, a env do projeto e dos
  serviços e as referências de secrets (o texto cifrado é copiado como está: é o mesmo servidor, a
  mesma chave). **Domínios e portas externas não são copiados** (são únicos no servidor); a tela
  pede os novos. Os vínculos e o `depends_on` são reescritos para apontar para as cópias.
- **Comparar** dois ambientes: serviço a serviço, o que difere (imagem/release no ar, variáveis,
  réplicas, recursos).
- **Promover**: escolher a release no ar de um serviço em homologação e pô-la no irmão em produção.
  **Só o código (a imagem) é promovido**; a configuração continua a de produção (é para ela ser
  diferente). Por baixo, é um deploy que pula o build usando o `image_ref` da release (§1.3).
  Pede confirmação.

### 2.4 A tela

- Lista de projetos: ambientes aparecem agrupados sob o projeto de origem, com o nome do ambiente
  como selo.
- Tela do projeto: botão **Criar ambiente** e, num projeto que tem ambientes, uma aba **Ambientes**
  com a comparação lado a lado e o botão **Promover** por serviço.

### 2.5 Fases

- **Fase 1:** campos novos, criar ambiente, comparar, promover, telas. *Implementada:* comandos
  `EnvironmentCreate`, `EnvironmentList`, `EnvironmentCompare` e `EnvironmentPromote` (o último nos dois
  sentidos: "Promover para a origem" e "Trazer da origem"); aba **Ambientes** no projeto nos dois
  clientes; selo "ambiente: X" e ambiente logo depois da origem na lista de projetos. A release
  promovida grava `promoted_from` (coluna nova em `release`). **[decisão minha]** A cópia também deixa
  de fora: política de backup, papel de servidor compartilhado e vínculos/dependências para fora do
  projeto (são os bancos compartilhados da origem). Jobs agendados nascem desligados. Na comparação,
  o id do zip enviado também é ignorado (cada lado tem o seu; a imagem é que conta), e os botões somem
  quando os dois lados já rodam a mesma imagem.
- **Depois:** **preview por PR** (ambiente efêmero criado pelo webhook, apagado quando o PR fecha),
  que precisa de domínio curinga (ACME DNS-01) — fica para um plano próprio.

### 2.6 Migração

Campos novos com `#[serde(default)]` no YAML. Projetos existentes ficam como estão (sem ambiente).
**Atenção:** uma versão anterior do daemon lê o YAML novo, mas ao regravar **perde** os campos que não
conhece. Não volte de versão depois de criar ambientes.

### 2.7 O que foi verificado (2026-11-24)

- Teste unitário da cópia (ids novos, reapontar dependência/vínculo/pré-deploy/gatilho, o que fica de
  fora, jobs agendados desligados, rede nova) e do filtro da comparação.
- Ponta a ponta num daemon isolado com Podman: origem com zip v1 → criar ambiente → deploy no ambiente
  a partir do **zip copiado** (serve "v1") → ambiente com v2 e MODE=b → comparar → promover para a
  origem: sem build, origem serve "v2", com MODE=a e o domínio dela intactos, release "promovida de …
  v2"; depois a comparação diz "mesma imagem"; promover entre serviços que não são irmãos é recusado.
- GUI (Xvfb): selo na lista de projetos e a aba comparando. Webui (Chrome headless): selo, aba da
  origem (lista e Criar) e do ambiente (comparação e botões).
- **Não verificado:** criar o ambiente **pela tela da GUI** (o Xvfb não entrega teclado; o comando foi
  verificado pela API e pela webui não); "Trazer da origem" na prática (é o mesmo comando no outro
  sentido).

---

## 3. Rota — o domínio como entidade própria

### 3.1 O problema

O domínio é um campo do serviço (`DomainRoute`: domínio, porta, TLS), e a tabela do proxy é
"domínio → backends do serviço" (`features/ingress/router.rs`). Por isso não dá para:

1. servir um domínio por **vários serviços separados por path** (`/` → front, `/api` → back);
2. ter um domínio **sem serviço**: `www.app.com` → `app.com`, domínio antigo → novo;
3. **trocar o domínio de serviço** de uma vez só (hoje: tira de um, põe no outro, com um buraco no meio);
4. pôr **senha** (basic auth) na frente de um painel.

### 3.2 O modelo

Uma **Rota** pertence a um projeto (**[decisão minha]**: os destinos têm de estar na rede daquele
projeto) e fica no YAML de infra, em `ProjectInfra.routes`:

- `id`, `domain`, `tls`;
- `action`: **ou** `Forward { rules }`, com cada regra `{ path_prefix, service_id, port, strip_prefix }`,
  **ou** `Redirect { to, permanent }`;
- `basic_auth: Option<{ user, password }>` (a senha cifrada `ENC[age:…]`, como os outros segredos).

No proxy, a entrada de um domínio passa a ter uma lista de regras: vence o **prefixo mais longo**.

**[decisão minha] Os domínios de dentro do serviço continuam existindo e funcionando**, sem migração
nesta entrega. As rotas são uma segunda fonte para a mesma tabela do proxy. Um mesmo domínio não pode
estar nos dois lugares (validação recusa com uma causa clara). Migrar tudo para rotas fica para
depois, quando as rotas tiverem rodado de verdade.

### 3.3 A tela

- A tela global **Ingress** (hoje só leitura) ganha **+ Nova rota**, e as rotas aparecem na tabela
  com o destino (`/ → front :3000`, `/api → api :8080`, `↪ app.com (301)`) e os filtros.
- **Editor de rota** (janela na GUI, modal na webui): domínio + TLS; "Encaminha para serviços" (tabela
  de paths, com seletor de serviço e porta) ou "Redireciona" (URL + 301/302); "Pedir usuário e senha".
- A aba **Domínios** do serviço mostra também as rotas que apontam para ele, com "editar rota".
- Apagar rota pede confirmação.

### 3.4 Fases

- **Fase 1:** modelo, proxy com path/strip/redirect/basic auth, telas. *Implementada:* `domain::Route`
  em `ProjectInfra.routes` (senha `ENC[age:…]`); comandos `RouteList`, `RouteSave` (cria com `id`
  vazio), `RouteDelete`; o proxy consulta as rotas antes dos domínios de serviço
  (`router::decide_custom`); a tabela do proxy passou a guardar os IPs no ar de **todo** serviço (antes
  só registrava os que tinham domínio), e é daí que a rota tira o destino na hora da requisição; boot
  leva as rotas ao proxy depois do recover; `ServiceUpdate` recusa domínio que é de rota e `RouteSave`
  recusa domínio que é de serviço. Telas: seção **Rotas próprias** e editor na tela Ingress (janela na
  webui, painel na GUI). **[decisão minha]** Rota **sem TLS** é servida em HTTP (antes, todo HTTP ia
  para HTTPS); os domínios de serviço continuam como eram. A aba Domínios do serviço **não** mostra as
  rotas que chegam nele (ficou para depois).
- **Depois:** certificado próprio (upload), migrar `ServiceSpec.domains` para rotas, rotas na aba
  Domínios do serviço.

### 3.5 O que foi verificado (2026-11-24)

- Testes unitários: prefixo mais longo e fronteira (`/api` não casa `/apix`), tirar prefixo, Location do
  redirect, validação da forma, e a decisão do proxy (encaminhar, redirect, senha, serviço fora do ar,
  esquecer serviço, linhas na foto do Ingress).
- Ponta a ponta no daemon isolado (HTTP em `127.0.0.1:18080`): a tabela do Ingress resolve o IP do
  serviço no ar; caminho de serviço parado → 502; caminho que não casa → 404; redirect 301 com o caminho;
  senha (401 sem, 401 errada, passa com a certa); a senha não volta e editar sem ela mantém a gravada;
  domínio repetido e domínio de serviço recusados nos dois sentidos; depois de reiniciar o daemon a rota
  continua; apagada, sai da tabela. GUI (Xvfb): lista, editor com os caminhos certos e salvar sem perder
  nada; webui (Chrome headless): lista, editor e criar uma rota com senha.
- **Não verificado:** o encaminhamento até o container de verdade (no Podman rootless desta máquina o
  host não alcança o IP do container, então o 200 fica só no teste unitário) e o certificado de uma rota
  com TLS.

---

## 4. Volume — os dados como entidade própria

### 4.1 O problema

Os dados de um serviço moram em volumes nomeados (quase sempre os de uma stack Compose). Mas:

- **Apagar um serviço não apaga os volumes** (`features/services/delete.rs`: `compose down` "NÃO
  remove os volumes") e não diz nada: eles ficam ocupando disco, sem dono, e só aparecem no
  inventário do Docker misturados com todo o resto.
- O backup (S3), o `.rpdata` e a recuperação são por **serviço**; não há um lugar que responda
  "quais dados eu tenho, de quem são, quanto ocupam e quando foi o último backup".

### 4.2 O modelo

**[decisão minha] Nesta entrega o Volume é uma visão calculada, não um registro novo no YAML.** O
daemon já sabe achar os volumes de cada serviço (é o que o backup e o export usam). O Volume junta:

- `name`, `size_bytes`, `service_id` dono (ou **nenhum** = órfão), `project_id`;
- `in_use` (algum container montado), `last_backup_at` e o resultado do último backup do dono;
- para órfãos: o nome do serviço que o criou, quando os labels permitirem descobrir.

Assim nada muda no formato do estado, e não há o que migrar.

### 4.3 O que dá para fazer

- **Tela Volumes** (global): tabela com nome, dono, tamanho, em uso, último backup; filtro "órfãos".
- **Apagar volume órfão**, com confirmação (e recusado se estiver em uso).
- **Ao apagar um serviço**, a confirmação passa a dizer quais volumes ficam para trás e oferece
  "apagar também os volumes" (desmarcado por padrão).

### 4.4 Fases

- **Fase 1:** visão calculada, `VolumeList`, `VolumeDelete`, opção no apagar serviço, telas.
  *Implementada, com uma mudança:* em vez de uma tela e comandos novos, a abstração **enriquece o
  inventário que já existia** (Runtime → Volumes, `DockerVolumes`, `RemoveVolume`): cada volume ganha o
  **dono** (serviço e projeto, pela etiqueta `rustploy.service_id`, pela stack Compose ou pelo nome que
  o spec monta), `orphan_of` e `has_backup_policy`; filtro **Só órfãos**. Ao apagar um serviço, uma
  segunda pergunta: apagar os dados ou manter os volumes (`ServiceDelete.delete_volumes`). Mantidos,
  eles ficam anotados na tabela `orphan_volume` com o serviço e o projeto de onde vieram, e é isso que
  aparece em `orphan_of`. **[decisão minha]** Na migração de banco (que apaga o serviço antigo) os
  volumes são mantidos. As etiquetas vêm da listagem de volumes, porque o `system df` do Podman não as
  traz.
- **Depois:** política de backup por volume, montar um volume órfão em outro serviço ("adotar").

### 4.5 O que foi verificado (2026-11-24)

- Teste unitário da tabela de órfãos.
- Ponta a ponta com uma stack Compose com volume nomeado: o volume mostra o serviço e o projeto; apagar
  o serviço mantendo os volumes → `orphan_of: "store… (rotas-…), 24/11/2026"`; `RemoveVolume` o apaga e
  esquece; outro serviço apagado com `delete_volumes` leva o volume junto. GUI (Xvfb): coluna DONO e
  filtro Só órfãos; webui: a mesma coisa.
- **Achado e corrigido de passagem:** na GUI em inglês, o botão **Remover** de Volumes, Imagens e Redes
  nunca aparecia: comparava o rótulo traduzido com `"SEM USO"`. Agora compara um campo não traduzido.
- **Não verificado:** a pergunta de volumes ao apagar serviço clicada na GUI (a mesma chamada foi
  verificada pela API); volume nomeado de serviço Application no Podman (ele trata o nome como pasta
  relativa; no Docker é volume nomeado).


### 4.6 Fase 2: montar, adotar e o volume nomeado que não funcionava (2026-11-24, v0.1.95)

O dono pediu "a questão dos volumes" com entrega autônoma. **[decisão minha]** Entendi como as pendências
da abstração Volume que apareceram na Fase 1:

1. **Volume nomeado em serviço Application estava quebrado nos dois motores.** O container era criado
   sempre com montagem *bind* (pasta do servidor), então `host_path = "pgdata"` falhava ("must be
   absolute" no Docker; pasta relativa ao serviço de API no Podman). Agora: caminho absoluto = bind;
   nome = volume nomeado, criado antes com a etiqueta `rustploy.service_id` (o dono aparece na lista).
2. **Adotar um órfão** (era o "Depois" do §4.4): `ServiceVolumeAttach{service_id, volume}` e
   `ServiceVolumeDetach{service_id, container_path}`, só para Application. Recusa: Compose, caminho
   relativo ou `/` no container, pasta do sistema/do Rustploy (`bind_allowed`), caminho repetido e
   volume que já é de **outro** serviço (pelo spec, pela etiqueta ou pela stack Compose: dois serviços
   no mesmo volume corrompem os dados). Adotar tira a anotação de órfão. Desmontar não apaga nada.
   Nenhuma das duas telas editava volumes de serviço antes: a seção **VOLUMES** entrou no topo da aba
   **Dados**, com "volume existente" (sem dono; órfãos primeiro; anônimos de 64 hex fora) ou um nome
   novo/pasta, o caminho no container e só leitura.
3. **Volume novo demorava até 5 minutos para aparecer** (cache do inventário): um ouvinte do barramento
   invalida o cache quando um deploy termina; montar e desmontar também invalidam.

**Ficou para depois:** política de backup **por volume** (o backup continua por serviço, com todos os
volumes dele).

**Verificado:** teste unitário das regras de nome/montagem; ponta a ponta no daemon isolado com Podman
(stack Compose grava um arquivo no volume → serviço apagado mantendo os volumes → órfão → serviço
Application adota em `/www/dados` → deploy → `cat` no container lê `dados-antigos`; o volume passa a ter
dono; volume novo nasce com a etiqueta e aparece logo depois do deploy; as cinco recusas; desmontar tira
só aquela montagem e os dados ficam). Telas: GUI no Xvfb (seção, lista com o órfão primeiro, painel de
exportação intacto) e webui no Chrome headless (seção e opções). **Não verificado:** montar pela tela da
GUI até o fim (o Xvfb não entrega teclado; o comando foi provado pela API) e o volume nomeado no Docker
(só Podman nesta máquina; a montagem `type: volume` é a mesma API nos dois).
---

## 5. Protocolo (resumo)

| Comando | Abstração | Destrutivo? |
|---|---|---|
| `ReleaseList { service_id }` | 1 | não |
| `ReleaseDiff { from, to }` | 1 | não |
| `ReleaseRollback { release_id }` | 1 | **sim** (sobrescreve o spec) |
| `EnvironmentCreate { project_id, name, … }` | 2 | não |
| `EnvironmentCompare { project_id, other_project_id }` | 2 | não |
| `EnvironmentPromote { release_id, target_service_id }` | 2 | **sim** |
| `RouteCreate` / `RouteUpdate` / `RouteDelete` | 3 | delete **sim** |
| `VolumeList` | 4 | não |
| `VolumeDelete { name }` | 4 | **sim** |
| `ServiceDelete { …, delete_volumes }` | 4 | já era |

Todos passam pela política do agente (`api/agent/policy.rs`): os de leitura no papel `observer`, os
outros em `operator`, e entram no manual do agente.

## 6. Como cada entrega é verificada

Para cada abstração: teste unitário do caso novo, `make lint` + `make test`, roteiro de ponta a ponta
num daemon de teste isolado (deploy → release → mudar config → deploy → voltar; criar ambiente →
promover; rota com path e redirect com `curl`; serviço apagado deixando volume órfão → aparece →
apagar), GUI no Xvfb com captura, webui no navegador. Depois o publish completo.

## 7. Decisões que tomei sem o dono

1. Release não copia valor de secret; voltar usa o valor atual (§1.2).
2. Voltar para uma release restaura o spec do serviço, mas não a env do projeto (§1.3).
3. Imagens das 3 últimas releases protegidas por tag (§1.4).
4. Release como selo na aba Deployments, não aba nova (§1.6).
5. Ambiente = projeto com vínculo de origem, não um nível novo dentro do projeto (§2.2).
6. Promover leva só a imagem, não a configuração (§2.3).
7. Rota pertence a um projeto; domínios dentro do serviço continuam valendo, sem migração (§3.2).
8. Volume como visão calculada, sem registro no YAML nesta entrega (§4.2); na implementação, dentro do
   inventário Runtime → Volumes que já existia, em vez de uma tela nova (§4.4).
9. Cópia do ambiente deixa de fora também backup, servidor compartilhado e vínculos/dependências para
   fora do projeto; ambiente não copia rotas (§2.5).
10. Rota sem TLS servida em HTTP; domínios de serviço sem mudança (§3.4).
11. Migração de banco mantém os volumes do serviço antigo (§4.4).
12. Ambiente, Rota e Volume publicados juntos numa versão só (cabeçalho).
13. "A questão dos volumes" = volume nomeado quebrado + adotar órfão + atualização da lista; backup por
    volume ficou para depois (§4.6).
