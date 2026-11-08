# Limites do agente (MCP): o que ele pode editar, e como fechar os buracos

> **Status (2026-11-08):** os itens **A, C, 0, B e D estão implementados** no daemon (sem release ainda); ver as
> seções de cada um e a seção 10 (decisões tomadas). Testados por testes automatizados (daemon 505 testes);
> **nenhum teste foi feito contra um servidor de verdade**, e a GUI e a webui não foram exercitadas com a
> chave de visitante.

## 1. De onde veio e o achado principal

A pergunta foi: *"um agente via MCP consegue editar qualquer valor do estado da infra? Há algo que ele não
consiga alterar?"* A primeira resposta foi "o agente só edita o que é seguro editar". **Isso estava
errado, e este plano começa corrigindo.**

O achado que muda tudo:

> As restrições do agente (não revelar secret, esconder senhas, recusar alguns comandos) **só existem no
> caminho do MCP** (`/mcp`, arquivo `api/agent/tools.rs`). A rota `POST /api/rpc` chama o `dispatch` direto
> (`api/http/mod.rs`, função `rpc`), **sem nenhuma dessas restrições e sem mascarar nada**. E as duas rotas
> usam **o mesmo token** (`[api].token`). Um agente com acesso a um terminal (`curl`) e esse token (que está
> no próprio cabeçalho da configuração do MCP) faz `SecretReveal` e lê todas as senhas em claro.

Ou seja: hoje as restrições são uma **regra de boa conduta da ferramenta MCP**, não uma **fronteira de
segurança**. Quem tem o token tem o servidor. O que o agente "não consegue" é o que a ferramenta MCP não
oferece, não o que o daemon impede.

Dois outros fatos confirmados, que pesam nos itens abaixo:

- O `host_path` de um volume vira um *bind mount* **sem validar o caminho** (`engine/containers/lifecycle.rs`:
  `source: Some(v.host_path)`). E um serviço Compose, ou o `compose` de um job, aceita YAML qualquer, sem
  nenhuma checagem de `privileged`, `docker.sock`, montagem de pastas do servidor etc. (não há validação de
  compose no daemon).
- A credencial de um provider git (e a de um secret usado em `credentials`) é **anexada a qualquer URL
  `https://`** que o serviço, o job ou o `state_git` informar (`features/deploy/git.rs::inject_token`,
  `features/state_git/mod.rs::auth_for`), sem conferir que o host é o do provider.

## 2. O que o agente não consegue alterar hoje (resumo)

- **Imutável depois de criado:** ids, datas, `network_name`, `compose_project`; o `trigger_service_id` de um
  job; um provider git (só criar/apagar); um banco gerenciado (senha e limites); o token do webhook (só
  regenerar).
- **Só de escrita, por desenho (no MCP):** valor de secret (não há leitura), valor `Plain` de variável com
  chave sensível (ele vê `***`), criar token do registry, exportar serviço com valores, resolver divergência do
  `state_git`, aplicar/desfazer atualização do daemon (só com `allow_agent_apply`).
- **Sem comando algum:** o `config.toml` inteiro; status, `live_container_id` e `desired_state` (só
  indiretamente); histórico de deploys; auditoria; certificados; conteúdo do registry (só apagar).

O que importa decidir está nos quatro itens abaixo e na base (seção 3).

## 3. A base: onde as restrições têm de morar (item 0, pré-requisito dos demais)

**Problema:** sem isto, os itens B e D são decorativos, porque o agente contorna pelo `/api/rpc`.

**Decisão do dono (2026-11-07): a chave atual vira a chave de visitante, e a chave mestra é opcional.**
O agente já usa a chave atual, então ele fica restrito sem ninguém reconfigurá-lo, e esquecer algo deixa o
agente **restrito** (falha fechada), não solto. A mestra passa a ser a chave das pessoas (GUI e webui).

| Estado do servidor | O que vale |
|---|---|
| **Sem chave mestra** (como hoje, e como fica depois de atualizar) | a chave atual vale para tudo; nada quebra |
| **Com chave mestra** | a chave atual vira **visitante** (restrita, em qualquer rota); a mestra vale para tudo |

- **Gerar a mestra é o ato que liga a proteção.** Algo como `rustployd access-key --master`, que grava
  `[api].master_token` no `config.toml`. Atualizar o daemon e a GUI, sozinho, não muda nada.
- O `/mcp` e o `/api/rpc` passam pela **mesma** função de checagem. As regras de `tools.rs`
  (`proibido_ao_agente`, `restaurar_mascaras`, a redação `redigir`) saem de lá e viram um passo do
  `dispatch` que recebe quem está chamando.
- **A mestra nunca sai por rota nenhuma da API.** `GET /api/access-key` passa a entregar a chave de
  visitante, que é a que a tela "Conectar um agente" copia.
- Para o **visitante**, recusar também `GET /api/events` (o snapshot traz as variáveis em claro).
- A GUI e a webui entram com a mestra. Quem continuar com a chave antiga depois de gerar a mestra vê
  "proibido" em ações que antes funcionavam; a mensagem de erro tem de dizer "esta é a chave de visitante,
  use a mestra".
- **Perdeu a mestra?** Edita-se o `config.toml` no servidor (o dono tem acesso a ele).
- O que o visitante pode fazer depende dos níveis do item D. Começar com o nível **operador** como padrão.

**Onde mexer:** `platform/config.rs` (campo novo), `api/http/mod.rs` (reconhecer o chamador e propagá-lo),
`api/dispatch.rs` e `api/agent/tools.rs` (uma só implementação das regras), o comando `access-key`, GUI e
webui (login com a mestra, texto de erro, tela de conexão), docs pt e en, manual do agente.

**Testes:** o mesmo comando proibido recusado por `/mcp` **e** por `/api/rpc` com a chave de visitante;
aceito com a mestra; `/api/events` recusado ao visitante; a mestra ausente da resposta de
`/api/access-key`; sem mestra configurada, o comportamento atual.

**Risco:** a GUI e a webui precisam de um login novo (uma vez por servidor) depois de gerar a mestra. É a
troca consciente de "o agente precisa mudar" por "a pessoa precisa mudar".

**Esforço:** o maior dos itens (daemon, duas telas e docs), mas é o único que transforma a lista de
proibições numa proteção real.

## 4. Item A: a credencial só vai para o host do provider — **IMPLEMENTADO (2026-11-07)**

**Hoje:** um agente (ou qualquer um com o token) cria um serviço com `Git { url: "https://servidor-dele/x.git",
provider_id: <o GitHub conectado> }`. No clone, o daemon monta `https://usuario:TOKEN@servidor-dele/...` e o
servidor dele recebe o token do GitHub (basta responder 401 ao `git`). O mesmo vale para o clone de um job e
para o `remote` do `state_git` (`StateGitConfigure`). O token de um **secret** usado em `credentials` vai pelo
mesmo caminho, o que **tira a proteção de "só de escrita"**: o agente nunca vê o secret, mas consegue mandá-lo
para fora.

**Feito:** `features/git_providers::{https_authority, provider_authority, credential_matches_url,
usable_token_for_url}`, usadas por `resolve_clone_credentials` (deploy, job e aquecimento) e pelo `state_git`
(recusa já na configuração do `remote`). Testes: `host_da_credencial_tests`. A proposta original era:
uma função só, `credential_host_ok(provider, url)`, usada nos três pontos
(`deploy/git.rs`, `jobs/runner.rs`, `state_git/mod.rs`). A regra: o host da URL tem de ser o do provider.

- GitHub público: o `base_url` vem vazio ou `https://github.com`, então o host é `github.com`.
- GitHub Enterprise e Gitea: o host do `base_url` do provider.
- Comparação por host (sem caixa, com porta), nunca por prefixo de texto (`github.com.evil.com`).
- Se não bater: **não anexa a credencial** e o clone segue sem ela (falha clara: "repositório privado
  inacessível") em vez de vazar.

**Ponto que continua aberto:** `credentials` (nome de um secret) não tem host associado, então não dá para
conferir. Opções: (a) aceitar o risco para pessoas e, para o agente, recusar `credentials` com URL fora dos
hosts dos providers cadastrados (depende do item 0); (b) acrescentar um campo opcional "host permitido" ao
lado de `credentials`. **Decisão do dono.**

**Testes:** a função com `github.com`, Enterprise, Gitea, `github.com.evil.com`, `https://github.com@evil.com`,
porta diferente, caixa; um clone de serviço e um `state_git` apontando para outro host **sem** credencial.

**Risco:** um provider configurado com um `base_url` diferente do host real do repositório deixa de
autenticar. É raro e o erro é claro; vale uma nota no changelog.

**Esforço:** pequeno. É o item mais barato por risco removido.

## 5. Item B: o agente não libera suspeita da auditoria

**Hoje:** `InfraAuditAllow` marca uma suspeita como "não é segredo" e tira a trava que impede valor em claro
de ir para o git. Não está na lista de proibidos do agente.

**Proposta:** proibir ao agente (`proibido_ao_agente`), como já é com `StateGitResolve`. Quem decide que algo
não é segredo é uma pessoa. Com o item 0, a proibição vale também pelo `/api/rpc`.

**Pergunta ao dono:** o agente deve poder **configurar** o `state_git` (`StateGitConfigure`)? Com o item A ele
não consegue mais desviar a credencial, mas ainda consegue ligar, desligar ou trocar o repositório. Sugestão:
deixar (é operação normal de infra), mas só depois do item A.

**Testes:** `InfraAuditAllow` recusado ao agente (MCP e `/api/rpc`); aceito para a pessoa.

**Esforço:** mínimo (uma linha e um teste), mas só vale de verdade com o item 0.

## 6. Item C: o `JobUpdate` passa a restaurar variáveis mascaradas

**Hoje:** o agente vê as variáveis sensíveis de um job como `***`. O `JobUpdate` substitui o job inteiro e
**não** devolve o valor real (o `ServiceUpdate` e o `ProjectEnvSet` devolvem). Resultado: o agente não
consegue editar nem ligar/desligar (`enabled`) um job que tenha senha em claro nas variáveis, porque a
regra "recusar `***`" o barra. Isso é um defeito, não uma decisão de segurança.

**Proposta:**
- Em `restaurar_mascaras` (`api/agent/tools.rs`), no braço do `JobUpdate`: buscar o job atual
  (`store::job::get`) e chamar `restaurar_envs(env_vars, &job.env_vars)` antes de `sobrou_mascara`.
- Com o item 0 isso vira parte do passo único do `dispatch`.
- Opcional, depois: um `JobEnvValueSet` (como o `EnvValueSet`), para editar um valor sem reenviar o job.

**Testes:** job com `DB_PASSWORD` em claro; `JobList` (mascarado) → `JobUpdate` só mudando `enabled` → a
senha continua intacta; valor novo para a chave sensível é gravado; `***` sem original continua recusado.

**Risco:** baixo. Só devolve o que o agente já enviou mascarado.

**Esforço:** pequeno.

## 7. Item D: bind mount e compose (o que o agente pode montar e rodar)

**Hoje:** quem cria ou altera um serviço escolhe imagem, comando e montagens. Com `volumes` (`host_path`) ou
com o YAML de um Compose, dá para montar `/etc/rustploy` (onde estão a chave mestra e a `age.key`) ou
`/var/run/docker.sock` num container próprio e ler tudo. Isso é inerente a um PaaS: **quem pode criar
containers arbitrários tem o servidor**. Não é um bug a corrigir com uma validação só.

**Por que uma lista de bloqueio não resolve:** validar `volumes` não basta (o Compose e o `compose` de job
também montam), e varrer o YAML atrás de `privileged`, `docker.sock`, `network_mode: host`, `pid: host`,
`cap_add`, `devices` e pastas sensíveis é uma lista que sempre terá um caminho esquecido. Serve para reduzir
risco, não para prometer segurança.

**Opções**, da mais fraca à mais forte:

| # | Opção | Protege? | Custo |
|---|---|---|---|
| 1 | Não fazer nada; documentar que o token do agente é acesso total | não | zero |
| 2 | Validar `volumes[].host_path` para todos (volume nomeado ou raiz permitida; negar `/etc/rustploy`, `/var/lib/rustploy`, `docker.sock`, `/`) | pouco (o Compose escapa) | pequeno |
| 3 | Varrer o Compose/`compose` de job por chaves perigosas, **só para o agente** | em parte (lista de bloqueio) | médio |
| 4 | **Níveis de agente**: "observador" (só leitura, deploy e reinício), "operador" (edita variáveis, secrets, domínios, réplicas, healthcheck de serviços **existentes**; não muda fonte, imagem, compose, volumes, comando) e "administrador" (como hoje) | **sim**, por construção | maior |

**Recomendação:** a opção 4, **construída sobre o item 0** (o nível viaja com o `agent_token`; pode haver um
token por nível). O nível "operador" cobre quase tudo que um agente faz no dia a dia (trocar variáveis,
secrets, reiniciar, olhar logs) sem poder criar nem alterar o que roda. A opção 3 pode vir depois, só como
camada extra para o "administrador". A 2 é barata e útil mesmo sozinha (um erro humano de digitar `/`
deixa de montar a raiz), mas não deve ser vendida como proteção contra o agente.

**Perguntas ao dono:**
- Quer níveis (observador, operador, administrador) ou basta um só nível "agente"?
- O "operador" pode **criar** serviços novos ou só mexer nos existentes?
- Existe algum caso real em que o agente precise alterar a fonte ou o compose de um serviço? Se sim, ele
  vira "administrador" e fica com o risco descrito.

**Testes:** cada nível contra uma tabela de comandos permitidos/recusados (um teste que lista **todos** os
comandos do `dispatch`, para um comando novo não nascer sem nível definido — o mesmo truque do teste de
cobertura do manual).

## 8. Ordem sugerida e dependências

```
Item 0 (papéis por token) ──┬─► Item B (InfraAuditAllow)         [depende do 0 para valer]
                            ├─► Item D (níveis do agente)        [depende do 0]
                            └─► Item C (JobUpdate), que entra no passo único do dispatch
Item A (host da credencial) ── independente, pode ir primeiro e sozinho
```

1. **A** primeiro: pequeno, independente, remove um vazamento real de credencial (inclusive para pessoas).
2. **C**: pequeno, corrige um defeito, pode ir junto do A.
3. **0**: a base. Maior mudança, mas sem ela B e D não protegem de verdade.
4. **B**: uma linha, depois do 0.
5. **D**: depois do 0, com as respostas às perguntas da seção 7.

**Versões:** A, C e B mexem só no daemon (patch). O 0 e o D mexem no daemon e nas duas telas (GUI e webui) e
não exigem mudança no `rustploy-shared` se o nível ficar no `config.toml`; se o nível viajar no protocolo,
é um `shared` novo.

## 9. O que este plano não resolve

- Quem tem o **token principal** continua com acesso total. O plano só cria um token mais fraco para o agente.
- O `config.toml` e o servidor em si continuam fora do alcance do protocolo, por desenho.
- O token de um **secret** em `credentials` continua sem host para conferir (ver item A).
- Se o agente rodar na **mesma máquina** do servidor, com acesso ao disco, o plano não o impede de ler os
  arquivos; ele cobre o agente que fala com o daemon pela rede.

## 10. Decisões

Respondidas em 2026-11-07:
- Fazer o plano, nesta ordem: A (feito), C, 0, B, D.
- Item 0: a chave atual vira **visitante**, a mestra é opcional e gerá-la liga a proteção (ver a seção 3).

Tomadas em 2026-11-08, **sem o dono ter respondido, usando as sugestões do plano** (revisar se discordar):
1. **Três níveis** (`observer`, `operator`, `admin`), com **`operator` como padrão** do visitante.
2. `credentials` (secret) com URL de fora dos providers: **não tratado**; continua aberto (o secret não tem
   host associado). Um visitante `admin` ainda consegue mandá-lo para fora.
3. Configurar o `state_git` (`StateGitConfigure`) e **criar** serviços novos: ficam em **`admin`** (mais
   estrito que a sugestão do plano de deixar para o operador). `StateGitSyncNow`, `Resume` e `DismissRemoval`
   são `operator`.
4. O operador **liga e desliga jobs** e edita as variáveis deles (`JobUpdate` é `operator`, com `compose`,
   `git_source` e `main_service` congelados), além do que o plano previa.

## 11. Como ficou (implementação, 2026-11-08)

- `api/agent/policy.rs`: `Level`, `Caller`, `required_level` (um `match` **exaustivo** sobre os 127 `Command`,
  sem curinga: um comando novo não compila até ter nível), `check`, e as listas de campos congelados do
  operador (`OPERATOR_FROZEN_SPEC_FIELDS`, `OPERATOR_FROZEN_JOB_FIELDS`).
- `api/http/mod.rs`: `Auth` (token, mestra, nível); `Auth::caller` reconhece quem chama; `/api/rpc` e `/mcp`
  passam o nível (via `task_local`) à mesma `tools::rpc`; `/api/events` e o upload de zip recusam o visitante
  (zip: abaixo de `admin`); a mestra nunca é devolvida por `/api/access-key` nem aceita em `/api/login`.
- `api/agent/tools.rs`: `InfraAuditAllow` proibido (B); `JobUpdate` restaura as variáveis mascaradas (C);
  `ServiceUpdate`/`JobUpdate` abaixo de `admin` não mudam o que roda.
- `[api].master_token` e `[api].visitor_level` no `config.toml`; `rustployd access-key --master` gera (256 bits,
  falha em vez de gerar fraco), grava e imprime o token de conexão da mestra.
- Docs pt e en: `instalacao/config-toml`, `agentes/seguranca`; manual do agente.

**Sem mestra configurada nada muda**, exceto: o MCP passa a recusar `InfraAuditAllow` (B) e a restaurar
máscaras no `JobUpdate` (C), para todo mundo.

## 12. O que ainda falta

- Exercitar na GUI e na webui com a chave de visitante (o que a pessoa vê ao ser recusada) e, num servidor
  real, gerar a mestra e conferir o fluxo inteiro (`access-key --master`, reiniciar, entrar com a mestra,
  agente com a antiga).
- `credentials` (secret) sem host associado (decisão 2).
- A GUI/webui não mostram o nível do visitante nem explicam o 403; hoje ele aparece como erro de comando.
