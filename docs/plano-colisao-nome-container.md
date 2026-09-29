# Plano: colisão de nome de container entre projetos

> **Status: implementado** (2026-09-29), itens 1, 2 e 3, com teste de ponta a
> ponta (abaixo, "Como foi validado").
>
> Diferenças em relação ao texto do plano, decididas na implementação:
> - o pedaço de ID no nome são os **últimos** 8 caracteres do ULID (parte
>   aleatória), não os primeiros (timestamp) — ver "Bug encontrado no teste";
> - réplicas live são achadas por label (`containers::find_live_replicas`),
>   não por nome recalculado: acha o formato atual, o legado e o container de
>   um serviço renomeado; staging é achado pelo nome exato do deploy, atual ou
>   legado (`containers::find_staging`), sempre conferindo o dono;
> - o alias entra no **início do `SwappingIn`** (réplica única) ou logo após o
>   healthcheck (rolling), sempre antes de o ingress conhecer o IP — reconectar
>   muda o IP e derrubaria conexões já roteadas.
>
> Bug confirmado por
> leitura de código. **Nenhuma colisão existe hoje** no servidor de produção
> (consulta no banco do rustploy, 2026-09-29) — por isso o plano vai direto à
> correção de fundo, sem fase de convivência com colisões.

## O problema, em termos simples

Nome de container Docker é **único no host inteiro**. O rustploy chama os
containers de serviços **Application** (Registry, Git, Archive) de
`rp_<nome_do_serviço>`, sem nenhum pedaço do ID
(`docker/containers.rs::replica_live_name`), mas a regra de nome único de
serviço vale **só dentro do projeto** (`db/services.rs`). Dois projetos com um
serviço `api` disputam o container `rp_api`.

Serviços **Compose** não têm o problema: o nome da stack já inclui 8
caracteres do ID (`shared::compose_project_name`).

## O que aconteceria (pior que um erro de "nome já existe")

O deploy acha o container antigo **pelo nome** e o destrói sem conferir o dono
(`deploy/executor.rs`, rolling update):

```rust
let live_name = containers::replica_live_name(&svc.spec.name, i); // "rp_api"
if let Ok(Some(old_cid)) = containers::find_by_name(&self.docker.inner, &live_name).await {
    let _ = containers::stop_graceful(&self.docker.inner, &old_cid, 30).await;
    let _ = containers::remove(&self.docker.inner, &old_cid).await;
}
containers::rename(&self.docker.inner, &staging_id, &live_name).await?;
```

Com A e B tendo um `api`: o deploy de B acha `rp_api` (de A), remove, e
promove o seu. **A cai em silêncio** (502), com o rustploy dizendo que está
`Running`. No reinício do daemon, a recovery de A acha o container de B e
marca A como `Stopped`; um redeploy de A derruba B — os dois se derrubam
alternadamente. Rollback e o fallback de `service_stop` também agem pelo nome.

## A causa de fundo

Não é só o nome curto. É que **o nome do container é usado como
identificador** — para achar "o live deste serviço", "a réplica 2", "o staging
deste deploy". Nome é para humanos e para DNS; identidade deveria vir de algo
que o rustploy controla e que não colide. Os containers já carregam labels
(`rustploy.managed`, `rustploy.service_id`, `rustploy.deployment_id`), só não
são usados nesses pontos.

## A correção

Três mudanças, que se completam:

### 1. Achar containers por label, não por nome

Um label novo em `containers::create_staging`: `rustploy.replica` = índice
da réplica (`0`, `1`, …). Não se cria label de papel (`live`/`staging`),
porque label de container é imutável no Docker e o papel muda na promoção. O
papel continua vindo do **sufixo do nome** (`_staging_<dep>`), que o rename da
promoção já remove — mas o sufixo só é olhado **entre containers já filtrados
por `rustploy.service_id`**, então nunca decide sobre container alheio.
(Containers de hoje não têm `rustploy.replica`; sem o label, vale o índice do
sufixo `_r<i>` do nome, e sem sufixo, `0`.)

**Regra:** toda busca de container de serviço Application começa por
`rustploy.service_id=<id>`. Uma função `containers::find_replica(docker,
service_id, role, idx)` substitui `find_by_name` em todos os pontos que hoje
montam `replica_live_name`/`replica_staging_name` para procurar: rolling
update, rollback, recovery e o fallback de `service_stop` (10 chamadas no
executor, 3 na recovery, 1 no stop). Com isso, **nenhum caminho do código
consegue mais tocar num container de outro serviço**, qualquer que seja o nome.

### 2. Nome do container com o ID

Como no Compose: `rp_<id8>_<safe>`, `rp_<id8>_<safe>_r<i>`,
`rp_<id8>_<safe>_staging_<dep>`. Uma função em `crates/shared`, ao lado de
`compose_project_name`, gera esses nomes (com testes), e `containers.rs` passa
a usá-la. Com o item 1, o nome deixa de ser identidade — mas continua
precisando ser único, senão o `create` falha; o ID garante isso.

### 3. `rp_<safe>` continua funcionando, como alias de rede

Quem hoje usa `http://rp_api:8080` numa env var (é o que o card "Internal URL"
mostra e o que `docs/internal-networking.md` ensina) não pode quebrar. Então
`rp_<safe>` vira um **alias na rede do projeto** (`EndpointSettings.aliases`).
Alias é por rede, não por host: `rp_api` em A e `rp_api` em B não colidem mais.

**Quando dar o alias** — testado num Docker local em 2026-09-29:

| Opção | Resultado |
|---|---|
| staging já nasce com o alias | o DNS de `rp_api` passa a devolver o **staging**, antes do healthcheck dele passar. **Descartada.** |
| alias dado na promoção: `network disconnect` + `network connect --alias rp_<safe>` | funciona; `rp_api` passa a resolver para o novo. **O IP do container muda** (no teste: `.3` → `.2`). |
| `docker rename` depois | o nome antigo (`…_staging_x`) some do DNS, o novo resolve. |

Então a promoção fica:

1. staging sobe **sem** alias, passa no healthcheck pelo IP (como hoje);
2. para e remove o live antigo (achado **pelo label**, item 1);
3. `disconnect` + `connect --alias rp_<safe>` do staging na rede do projeto;
4. `rename` para `rp_<id8>_<safe>`;
5. **relê o IP** e só então atualiza o ingress. Hoje o executor usa o IP
   lido antes do healthcheck; depois do passo 3 ele não vale mais.

A janela sem `rp_api` resolvendo é a mesma de hoje (entre remover o antigo e
renomear o novo) — o disconnect/connect acrescenta milissegundos.

**Bônus:** com réplicas, todas recebem o mesmo alias `rp_<safe>`, e o DNS do
Docker distribui entre elas. Hoje `rp_api` aponta só para a réplica 0; as
outras (`rp_api_r1`…) só recebem tráfego pelo ingress.

### Transição dos containers existentes

Os containers atuais se chamam `rp_<safe>` e **não têm** os labels novos, mas
têm `rustploy.service_id` — e, como não há colisões hoje, cada `rp_<safe>`
pertence sem ambiguidade a um serviço só.

- Com a regra do item 1 (filtrar por `rustploy.service_id`), o container
  antigo é achado **pelo label**, independentemente do nome. O primeiro deploy
  de cada serviço depois da atualização remove o `rp_api` antigo e promove um
  `rp_<id8>_api` com alias — a migração acontece sozinha, deploy a deploy.
- Até o primeiro deploy, o container antigo segue no ar, resolvendo por
  `rp_api` como nome de container. Nada quebra no meio.
- Containers muito antigos, sem `rustploy.service_id` (o comentário de
  `service_stop.rs` sobre "antes da migração de prefixos de ID"): o daemon
  loga um aviso no boot, e o operador faz um deploy manual deles. Não há
  fallback por nome — é exatamente o fallback que causava o problema.

Nada precisa ser feito à mão no servidor.

### O que fica de fora

- **Rename de serviço.** Com o alias vindo do nome, renomear `api` → `backend`
  troca o alias no próximo deploy (`rp_api` deixa de existir). É o
  comportamento de hoje e continua igual; o que muda é que agora é explícito.
- **Rename de serviço Compose.** `compose_project_name` usa o nome, então
  renomear muda a stack e pode criar volume novo e vazio (suspeita registrada
  em `plano-banco-compartilhado.md` §2.4). É outro bug, mesmo espírito —
  identidade vinda do nome —, e merece plano próprio.
- **Unicidade global de nome.** Não é necessária: com as três mudanças, `api`
  em dois projetos passa a ser legítimo.

## Onde mexe

| Peça | Onde |
|---|---|
| Nomes com ID (+ testes) | `crates/shared/src/lib.rs`, junto de `compose_project_name` |
| Labels novos, `find_replica`, alias na criação/promoção | `crates/daemon/src/docker/containers.rs` |
| Rolling update, promoção (passos 1–5), rollback | `crates/daemon/src/deploy/executor.rs` |
| Recovery por label | `crates/daemon/src/deploy/recovery.rs` |
| Stop sem fallback por nome | `crates/daemon/src/api/handlers/service_stop.rs` |
| Doc | `docs/internal-networking.md` (hoje desatualizado também para Compose) |

GUI e webui não mudam: o card "Internal URL" de Application mostra `rp_<safe>`,
que continua resolvendo (agora como alias).

## Ordem sugerida

1. Item 1 sozinho já fecha o risco: com busca por label, nenhum deploy toca
   container alheio. Pode ir para produção isolado.
2. Itens 2 e 3 juntos (nome com ID sem o alias quebraria o `rp_<safe>`).
3. Teste de ponta a ponta num Docker real, com dois projetos com um serviço
   `api` cada: deploy alternado dos dois, reinício do daemon, rollback,
   réplicas > 1.

Se o banco compartilhado for implementado depois, este plano vem antes: os
dois mexem no executor e na recovery.

## Como foi validado (2026-09-29)

Um `rustployd` isolado (banco, chave e portas próprios), dois projetos com um
serviço Application de **mesmo nome** (imagem `redis:6.2-alpine`, healthcheck
TCP), operado pela API:

1. **Transição:** um live legado `rp_<safe>` com label do serviço A no formato
   antigo (sem `svc_`) é trocado, no primeiro deploy, por `rp_<id8>_<safe>`; o
   alias `rp_<safe>` resolve na rede de A para o container novo.
2. **O bug original:** deploy de B não toca no container de A; `rp_<safe>`
   resolve para B na rede de B e continua em A na de A.
3. Redeploy de A não toca em B.
4. Restart do daemon: a recovery marca os dois `Running`.
5. B com 2 réplicas: `…_api` e `…_api_r1`, alias nas duas, o DNS devolve as duas.
6. Rollback (healthcheck HTTP contra o redis falha): só o live antigo de A
   sobra, rodando; B intacto.
7. Stop de A para só A.

## Bug encontrado no teste, fora do escopo: rede de projeto compartilhada

A primeira rodada do teste falhou nos passos de DNS: **os dois projetos
receberam a mesma rede**. O nome da rede é `rp_net_<8 primeiros chars do ULID
do projeto>` (`docker/networks.rs::id_short`), e os 8 primeiros caracteres de
um ULID são timestamp: só mudam a cada ~1 s (sobram 10 bits de milissegundo).
O teste criou os projetos com ~100 ms de diferença:

```
prj_01M3NV213V06RX7CXJ5FDDAP0K  → rp_net_01M3NV21
prj_01M3NV215T5XPM5GME8331T0RF  → rp_net_01M3NV21
```

Consequência: **dois projetos criados no mesmo segundo dividem a rede** — os
containers de um enxergam os do outro, e os aliases `rp_<safe>` se misturam
(no teste, `rp_api` resolvia para A e B nas duas "redes"). Criando projetos à
mão pela interface isso é improvável; um **import de manifesto** com vários
projetos cria todos em sequência e cai exatamente nisso.

O mesmo prefixo de timestamp está em `shared::compose_project_name` (`rp_<id8>_<safe>`
de stacks Compose): dois serviços Compose de **mesmo nome** criados no mesmo
segundo, em projetos diferentes, teriam o mesmo nome de projeto Compose — o
Compose trataria os dois como uma stack só.

Para conferir no servidor:

```sh
sqlite3 /var/lib/rustploy/db/rustploy.db "
  SELECT substr(id, 5, 8), group_concat(name, ', ')
  FROM project GROUP BY substr(id, 5, 8) HAVING count(*) > 1;"
```

Resultado em produção (2026-09-29): **nenhum** par de projetos com o mesmo
prefixo — a rede compartilhada não acontece hoje — e **nenhum** par de
serviços Compose com mesmo prefixo + mesmo nome (consulta abaixo, que compara
por `lower(name)`, aproximação do `normalize_name`). Até a correção, evitar
import de manifesto que crie vários projetos ou serviços de uma vez.

```sh
sqlite3 /var/lib/rustploy/db/rustploy.db "
  SELECT substr(id, 5, 8) || '_' || lower(name), group_concat(project_id, ', ')
  FROM service
  WHERE json_extract(spec, '$.source.Compose') IS NOT NULL
  GROUP BY 1 HAVING count(*) > 1;"
```

**Plano da correção: `docs/plano-nome-gravado-rede-e-stack.md`** — grava o
nome na criação em vez de derivá-lo, sem renomear nada que já existe. (O
parágrafo abaixo é a análise original, de antes desse plano.)

Corrigir exige **renomear redes (e stacks Compose) existentes** — mexe em
containers em produção e, no caso do Compose, no nome dos volumes. Precisa de
plano próprio; não foi feito aqui. O teste E2E espera 1,2 s entre os projetos
para não esbarrar nisso.
