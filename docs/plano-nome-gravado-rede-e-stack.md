# Plano: nome de rede e de stack Compose gravados, não derivados

> **Status: implementado** (2026-09-29), passos 1 a 3 da "Ordem sugerida".
>
> Diferenças em relação ao texto do plano, decididas na implementação:
> - `Service` ganhou `compose_project: Option<String>` (serialização com
>   `#[serde(default)]`, só leitura) e o helper `Service::compose_project_name()`,
>   que devolve o gravado ou, na falta, a fórmula legada. `Project` **não**
>   ganhou campo: a rede é lida por `db::projects::network_name` /
>   `network_names`.
> - a rede dos projetos existentes é preenchida em SQL na própria migração
>   (`db::migrate`); a stack dos serviços Compose precisa do Docker, então é
>   preenchida logo depois da conexão com o Docker no boot
>   (`db::services::backfill_compose_projects`, chamada em `main.rs`), antes da
>   recovery. Sem Docker listável, as colunas ficam `NULL` e vale o fallback
>   legado (comportamento de antes).
> - o backfill nunca adota a stack de outro serviço. O prefixo do nome não basta
>   (8 primeiros chars do ID: igual para tudo criado no mesmo segundo, como num
>   import de manifesto — ver "Bug achado no teste com a infra de produção"): uma
>   stack só é adotada por um serviço renomeado se as chaves de serviço dos seus
>   containers (`com.docker.compose.service`) cabem nas do compose do serviço e
>   só ela serve; senão vale a fórmula legada com o nome atual.
> - `update_spec` grava a stack de um serviço que **virou** Compose numa edição
>   de fonte, com o nome do momento; se já há stack gravada, não mexe (é o que
>   torna o rename seguro).
> - o `rustploy-import` insere direto no banco sem as colunas novas: ficam
>   `NULL` e o daemon preenche no boot (rede pela fórmula legada).
> - **validado com Docker de verdade** em 2026-09-29 (seção "Como foi validado"),
>   inclusive com o manifesto de produção (`rustploy-infra.zip`) importado por um
>   daemon do commit anterior. Não foi feito contra uma cópia do `rustploy.db` de
>   produção.

## O problema, em termos simples

O rustploy dá nome a duas coisas do Docker **calculando o nome toda vez** a
partir do ID (e, numa delas, do nome do serviço):

| Coisa | Nome | Onde |
|---|---|---|
| Rede do projeto | `rp_net_<8 primeiros chars do ID do projeto>` | `docker/networks.rs::project_net_for` |
| Stack Compose de um serviço | `rp_<8 primeiros chars do ID do serviço>_<nome do serviço>` | `shared::compose_project_name` |

Isso tem dois defeitos.

**1. Os 8 primeiros caracteres de um ID não são únicos.** Os IDs são ULIDs, e
um ULID começa pelo horário da criação. Os 8 primeiros caracteres só mudam a
cada ~1 segundo. Então:

- dois **projetos** criados no mesmo segundo recebem **a mesma rede** — os
  containers de um enxergam os do outro, e os nomes internos (`rp_api`) se
  misturam. Foi o que o teste E2E mostrou;
- dois **serviços Compose de mesmo nome**, em projetos diferentes, criados no
  mesmo segundo, recebem **a mesma stack** — o Compose trata os dois como uma
  coisa só, e um deploy de um mexe nos containers (e volumes) do outro.

Pela interface, criando uma coisa de cada vez, é quase impossível acontecer.
Um **import de manifesto** cria tudo em sequência, em milissegundos — e cai
exatamente nisso.

**2. Nome que depende do nome do serviço muda quando o serviço é renomeado.**
A stack Compose leva o nome do serviço. Renomear `postgres` → `banco` muda o
nome da stack no próximo deploy, e o Compose, achando que é uma stack nova:

- sobe containers novos, deixando os antigos rodando órfãos;
- cria **volumes novos, vazios** — o nome do volume é `<stack>_<volume>`.

Para um banco de dados, isso parece "perdi todos os dados" (os dados estão lá,
no volume antigo, mas o serviço não os vê mais). Isto estava registrado como
suspeita em `plano-banco-compartilhado.md` §2.4; o código confirma
(`compose_project_name` recebe `svc.spec.name` em todos os usos).

## A causa comum

Nos dois casos, o nome é **derivado** a cada uso — de algo que não é único (o
começo do ULID) ou que muda (o nome do serviço). Um nome de recurso do Docker
precisa ser as duas coisas: **único** e **estável**. A forma mais simples de
garantir estabilidade é **gravar o nome uma vez, na criação, e sempre lê-lo
de lá**.

## A correção

### 1. Duas colunas novas

| Tabela | Coluna | Conteúdo |
|---|---|---|
| `project` | `network_name TEXT` | nome da rede Docker do projeto |
| `service` | `compose_project TEXT` | nome da stack Compose (só serviços Compose; `NULL` nos demais) |

Ficam em **colunas próprias, fora do `spec`** de propósito: o `spec` é o JSON
que a GUI e a webui leem, editam e mandam de volta no `ServiceUpdate`. Um campo
ali correria o risco de um cliente não o preservar e apagá-lo numa edição —
e o nome voltaria a ser derivado. Nas colunas, nenhum cliente consegue tocar.

Um índice `UNIQUE` em cada uma (ignorando `NULL`) faz o banco recusar,
na criação, qualquer nome repetido — em vez de a colisão aparecer depois, no
Docker, em silêncio.

### 2. O que acontece com o que já existe: nada

Na subida do daemon, a migração **preenche** as colunas vazias com **o nome que
o recurso já usa hoje**:

- `project.network_name` ← `rp_net_<8 primeiros chars>` (a fórmula atual);
- `service.compose_project` ← o nome da stack **que está de fato no Docker**
  para aquele serviço (label `com.docker.compose.project` dos containers com o
  prefixo `rp_<id8>_`); se não houver stack rodando, a fórmula atual com o nome
  atual do serviço.

Ler do Docker no caso da stack cobre o serviço que já foi renomeado e ainda não
foi redeployado: a stack viva é a do nome antigo, e é ela que tem os volumes.

Resultado: **nenhuma rede, stack, container ou volume em produção muda de
nome.** Tudo continua exatamente onde está. A migração só anota.

A fórmula antiga não é apagada: vira `legacy_*` e fica usada **só** pelo
preenchimento.

### 3. O que muda para projetos e serviços novos

Na criação (pela interface, pela API, pelo import de manifesto — todos passam
por `db::projects::create` / `db::services::create`):

- **rede:** `rp_net_<ID do projeto inteiro, minúsculo>` — ex.:
  `rp_net_01m3nv213v06rx7cxj5fddap0k`. Ninguém digita esse nome, então o
  tamanho não importa, e com o ID inteiro a colisão é impossível;
- **stack Compose:** `rp_<últimos 8 chars do ID>_<nome do serviço na
  criação>` — o mesmo esquema dos containers de Application (a parte
  aleatória do ULID, não o horário). É o nome que aparece nos containers
  (`rp_x7k2m9qa_banco-rp_banco-1`), então vale mantê-lo curto. Se o índice
  `UNIQUE` recusar (chance de ~1 em 10¹²), usa-se o ID inteiro.

### 4. Rename de serviço Compose deixa de perder volume

Com o nome da stack gravado, **renomear o serviço não muda a stack**. Os
containers e volumes continuam os mesmos. O único efeito visível é cosmético: a
stack de um serviço renomeado de `postgres` para `banco` continua se chamando
`rp_…_postgres`. É o preço certo — o nome do recurso deixa de acompanhar o
nome de exibição, que é justamente o que dá estabilidade.

(O alias `rp_<nome>` dos serviços **Application** continua acompanhando o
nome, porque é um hostname que o usuário usa em env vars. Isso já era assim e
não envolve volume nenhum.)

### 5. Todos os usos passam a ler o nome gravado

| Onde | Hoje | Depois |
|---|---|---|
| `deploy/executor.rs` (`network_name`, 2× `compose_project_name`) | calcula | lê do projeto / serviço |
| `deploy/recovery.rs` (2× rede, 1× stack) | calcula | lê |
| `api/handlers/service_stop.rs` (rede, stack) | calcula | lê |
| `api/handlers/logs_get.rs` (stack) | calcula | lê |
| `docker/containers.rs` (índice de containers por stack) | calcula | lê |
| `jobs/runner.rs` (rede do projeto do job) | `ensure_project_network(project_id)` | recebe o nome gravado |
| `docker/networks.rs::ensure_project_network` | calcula a partir do ID | recebe o nome |
| `api/handlers/docker_inventory.rs` (qual projeto é dono de cada rede) | tira o prefixo `rp_net_` e compara 8 chars | compara com a coluna, nome inteiro |

A função `networks::project_net_for` e `shared::compose_project_name` deixam de
ser chamadas fora da migração. `Service` (o modelo que o daemon devolve)
ganha `compose_project: Option<String>` só para leitura, útil para a GUI/webui
mostrarem o nome real da stack — nenhuma das duas precisa mudar para o plano
funcionar.

## E se já existisse uma colisão?

Não existe em produção (verificado). Mas um servidor qualquer poderia ter:

- **Dois projetos na mesma rede:** a migração preencheria os dois com o mesmo
  `network_name` e o índice `UNIQUE` falharia. Nesse caso a migração **não**
  cria o índice, loga um erro nomeando os projetos, e segue — o daemon sobe e
  funciona como hoje. Separar exige redeploy dos serviços de um dos projetos
  numa rede nova; fica como passo manual documentado, não automático, porque
  mexe em containers vivos.
- **Duas stacks com o mesmo nome:** mesmo tratamento. Separar exige decidir de
  quem são os volumes — não dá para automatizar com segurança.

## Como validar

- **Unitário:** a migração preenche com o nome atual; projeto novo recebe
  `rp_net_<ULID inteiro>`; serviço Compose novo recebe o nome com a parte
  aleatória; o `UNIQUE` recusa repetido.
- **E2E** (reaproveitando o script do plano de colisão, que hoje espera 1,2 s
  entre os projetos para contornar este bug): **tirar a espera** e confirmar
  que dois projetos criados juntos recebem redes diferentes.
- **E2E de rename:** serviço Compose com volume, gravar um arquivo no volume,
  renomear o serviço, redeployar, conferir que o arquivo continua lá e que não
  sobrou stack órfã.
- **Daemon atualizado sobre um banco antigo:** subir o daemon novo sobre uma
  cópia do banco de antes, com containers rodando, e conferir que nenhum nome
  mudou e que o primeiro deploy de cada serviço reusa a mesma rede e stack.

## Ordem sugerida

1. Colunas + migração de preenchimento + leitura em todos os usos (§1, §2, §5),
   **sem** mudar ainda o formato dos nomes novos. Nesse ponto nada muda de
   comportamento — é o passo mais importante de testar contra o banco de
   produção (uma cópia).
2. Formato novo para criações (§3). A partir daqui import de manifesto volta a
   ser seguro.
3. Testes de rename (§4) e o E2E sem a espera.

Até o passo 2 entrar: evitar import de manifesto que crie vários projetos ou
serviços de uma vez.


## Como foi validado (2026-09-29)

`rustployd` isolado (banco, portas e config próprios), operado pela API, com
`redis:6.2-alpine` e um volume nomeado. Os recursos `rp_*` que já existiam no
host não foram tocados (a limpeza remove só o que difere de um snapshot tirado
antes).

**Daemon novo, banco limpo**
1. Dois projetos criados em seguida, sem espera, com IDs de mesmos 8 primeiros
   caracteres: `network_name` diferentes (`rp_net_<ID inteiro>`), uma rede cada,
   e um serviço `api` em cada projeto sem se misturar.
2. Serviço Compose com volume: grava um arquivo, renomeia `postgres` → `banco`,
   redeploy. Mesma stack (`rp_<id8>_postgres`), mesmo volume, um só container
   (sem órfão), arquivo intacto.

**Daemon do commit anterior cria tudo; daemon novo sobe sobre o mesmo banco**
3. Nenhum container, rede ou volume muda de nome, e nenhum container é
   recriado; os serviços voltam `Running` pela recovery.
4. `network_name` preenchido com o nome legado (`rp_net_<8 chars>`);
   `compose_project` preenchido com a stack viva. Um serviço **renomeado antes
   da atualização e ainda não redeployado** adota a stack do nome antigo.
5. Redeploy de tudo pelo daemon novo reusa as mesmas redes e volumes, com os
   dados intactos; um segundo rename + redeploy também.
6. Banco antigo **com colisão de rede**: o índice `UNIQUE` não é criado, o aviso
   é logado e o daemon segue no ar.

**Com a infra de produção** (`rustploy-infra.zip`: 1 projeto, 13 serviços, 6 Compose
e 7 Git), importada por um daemon do commit anterior num banco isolado.
Sobem só `cache`, `broker` e `storage` (Compose autocontidos); os de Git (repos
privados) ficam importados e parados; `db` e `mongo` não sobem (`host_port` +
volume externo abririam portas no firewall do host). O `broker` é renomeado
para `mq` **sem** redeploy antes de o daemon novo subir.
7. Nada muda de nome nem é recriado; `network_name` do projeto = nome legado;
   os 6 serviços Compose ganham stack, os 7 de Git não; `mq` adota a stack viva
   `rp_<id8>_broker`.
8. Redeploy dos três pelo daemon novo reusa redes e volumes, com os dados
   intactos; rename de `cache` + redeploy também.

## Bug achado no teste com a infra de produção

A primeira versão do backfill escolhia, para um serviço renomeado, "a única
stack viva com o mesmo prefixo de ID". O import cria os 13 serviços no mesmo
segundo, então **todos** têm o mesmo prefixo: o serviço `db`, que nunca subiu,
adotou a stack viva do `broker`, e o `mq` (ex-`broker`) ficou sem ela e foi
marcado `Stopped` pela recovery — o serviço "perdia" o volume e o outro passava
a mexer nos dados errados. Os testes de unidade e os E2E anteriores não pegaram
porque criavam poucos serviços e em segundos diferentes.

Correção: casar também pelas chaves de serviço do compose (seção acima) e só
adotar com candidato único; teste de regressão
`backfill_de_import_no_mesmo_segundo_liga_cada_stack_ao_seu_servico`.

Limite que continua: dois serviços Compose de mesmo prefixo **e** mesmas chaves
de serviço, ambos renomeados sem redeploy, são ambíguos — o backfill loga um
aviso e usa a fórmula legada (o que o daemon já fazia antes deste plano), sem
adivinhar.

## Segundo zip, de outro servidor (2026-09-29)

4 projetos e 9 serviços (3 Compose: `db` do Standimob — uma stack Supabase de 9
containers —, `rdo-banco` e outro `db` do projeto gestão; 6 Git, de repos
privados, que ficam parados). O firewall do host não foi tocado
(`RUSTPLOY_FW_SOCKET` apontado para um socket inexistente).

**Importado direto pelo daemon novo** (o caso de uso): 4 redes e 3 stacks
distintas, inclusive para os **dois serviços chamados `db`**; cada stack na rede
do seu projeto e sem vazar para a de outro; rename + redeploy sem volume ou rede
nova, com o marcador de todos os volumes intacto.

**Importado e implantado pelo daemon anterior, depois atualizado**: aqui o
import cria os 9 serviços no mesmo segundo, e a fórmula antiga dá a **mesma
stack** (`rp_<id8>_db`) aos dois `db`, e a **mesma rede** aos 4 projetos. O
daemon anterior então trata os dois `db` como uma stack só (o `reconcile` marcou
o segundo como `Running` só porque achou os containers do primeiro na stack).
O daemon novo sobe sobre esse banco sem renomear nem recriar nada, grava os
nomes que já existiam, não cria os índices `UNIQUE`, loga o aviso e segue no ar
— como o plano prevê: **ele não conserta uma colisão que já existe**. Separar
continua manual (redeploy de um dos projetos numa rede nova; decidir de quem são
os volumes da stack repetida). Serviços e projetos criados dali em diante já
saem com nomes distintos.

Observação do teste: o boot do daemon novo re-enfileira deploys que estavam
pendentes no antigo (recovery, comportamento já existente); numa rodada isso
criou um container que não existia no snapshot anterior, mas com o nome da
stack legada, ou seja, sem mudar nenhum nome.

Consequência prática: **importe zips de outros servidores pelo daemon novo**.
Importar pelo antigo e atualizar depois preserva o que existe, mas carrega as
colisões junto.
