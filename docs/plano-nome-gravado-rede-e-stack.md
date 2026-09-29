# Plano: nome de rede e de stack Compose gravados, não derivados

> **Status:** proposta, **não implementado** (2026-09-29).
> Origem: bug achado no teste E2E de `plano-colisao-nome-container.md`
> (seção "Bug encontrado no teste"). Produção verificada no mesmo dia:
> **nenhuma** colisão existente, nem de rede nem de stack.

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
