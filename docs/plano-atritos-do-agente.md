# Plano: os atritos que um agente encontrou subindo um projeto do zero

> **Status:** implementado no daemon 0.1.76 (v0.1.92) e completado no 0.1.77 (v0.1.93),
> na entrega autônoma de 2026-11-24.
> Só o daemon muda; GUI e webui não têm tela nova (ver "Os dois clientes").

## De onde veio

Em 2026-11-24 um agente (Claude Code, pelo MCP do Chiquitos) subiu um projeto
novo inteiro sem ninguém abrir a GUI: Postgres, Redis, uma API Rust vinda do
GitHub, vínculos, secrets gerados, um job de migração no pré-deploy e um
healthcheck HTTP. Funcionou, mas o caminho teve cinco tropeços. Nenhum deles
impediu o trabalho, mas cada um custou tentativa e erro, e dois deles deixariam
uma pessoa sem saber o que aconteceu. Este plano conserta os cinco.

## Os cinco pontos

### 1. O job de pré-deploy que nunca terminava

**O que aconteceu.** O deploy da API ficou mais de dez minutos em
`PreDeployCheck`. O container do job de migração já tinha **saído com código 0**
(`docker ps -a` mostrava `Exited (0) 4 minutes ago`), mas o rustploy continuava
esperando. Abortar o deploy liberou a fila, porém o `job_run` ficou para sempre
"em execução" no histórico, e o `JobRunCancel` respondia que ele não existia.

**Por quê.** São três defeitos que se somam:

1. `run_once_up` espera o `docker compose up --abort-on-container-exit` fechar os
   pipes de saída. Se o compose não perceber que o `main_service` saiu (é o que
   aconteceu no servidor, que roda Podman: o compose não recebeu o evento de
   saída e nem chegou a transmitir a saída do container), o daemon espera para
   sempre. Não havia nenhum vigia além dos pipes.
2. O processo `docker compose` era criado sem `kill_on_drop`. Abortar o deploy
   derruba a future do executor, mas o processo filho continuava vivo.
3. No caminho de pré-deploy, o `job_run` é aberto pelo executor do deploy. Se o
   deploy é abortado no meio, ninguém fecha esse `job_run`: ele só seria
   "consertado" no próximo boot do daemon (`recover_interrupted`).

**A correção.**

1. **Vigia do `main_service`.** Enquanto o compose roda, o daemon olha o
   container do `main_service` a cada 5 s (pelos rótulos
   `com.docker.compose.project`/`com.docker.compose.service`, que o compose
   põe nos dois motores). Se o container saiu e o compose **continua vivo 30 s
   depois**, o daemon conclui que o compose travou: guarda a saída do container
   no log do job (via `logs`, porque o compose não a transmitiu), escreve uma
   linha explicando o que houve, mata o compose e usa o **exit code do
   container** como resultado. O `down` de sempre limpa o resto. Os 30 s de
   carência existem para não competir com o caminho normal: quando o compose
   funciona, ele sai sozinho bem antes.
2. `kill_on_drop(true)` no processo do compose.
3. Um "guarda" no gate de pré-deploy: se a future for derrubada (deploy
   abortado) antes de o job terminar, o `job_run` é fechado como **cancelado**
   (o mesmo exit code `-2` do `JobRunCancel`), com uma linha no log dizendo que
   o deploy foi abortado, e o evento `JobRunStateChanged` sai normalmente.

4. **(0.1.77) O compose morre inteiro, e o abort limpa o que sobra.** O teste de ponta
   a ponta da 0.1.76 mostrou que matar o processo do compose não bastava: com Podman o
   comando é `podman --remote compose …`, que roda o plugin `docker-compose` como
   **neto**. O `start_kill`/`kill_on_drop` matava só o filho, e o plugin ficava órfão
   para sempre. Isso valia também para o `JobRunCancel` de antes desta entrega. Agora o
   compose nasce num grupo de processos próprio (`process_group(0)`) e o
   `ComposeGroupGuard` manda SIGKILL ao grupo inteiro quando o compose é abandonado
   (vigia, cancelamento ou future derrubada). No abort do deploy, o guarda do pré-deploy
   também remove os containers do projeto compose do run (pelo rótulo, sem o arquivo)
   e o diretório de build, que o `down` do `run_once` não chegou a limpar.

**Por que não mudar o compose para `-d` + `wait`.** Seria trocar o mecanismo
inteiro de execução de jobs (logs ao vivo, cancelamento, aquecimento) por causa
de um defeito do motor. O vigia é aditivo: não muda nada quando o compose se
comporta, e resolve quando não se comporta.

### 2. O container que morre no staging sem deixar log

**O que aconteceu.** O primeiro deploy da API falhou com
`container não encontrado na rede rp_net_… (presentes: [...])`. Nenhum log do
container: nem no build log, nem em `LogsHistory`, nem em `LogsExecutions`.

**Por quê.** A captura do log de um container que morre no staging já existe,
mas mora dentro de `poll_healthcheck`. Antes dele, `step_healthcheck_polling`
pede o IP do container na rede do projeto. Um container que já saiu é
**retirado da rede** pelo motor, então a busca do IP falha primeiro, com uma
mensagem sobre rede, e o rollback remove o container antes de alguém ver por
que ele morreu.

**A correção.** Antes de pedir o IP, `step_healthcheck_polling` inspeciona o
container. Se ele não está rodando, o deploy falha com uma mensagem que diz isso
(`o container saiu com código N antes do healthcheck`), e as últimas 50 linhas
dele entram no build log, com o mesmo formato que o `poll_healthcheck` já usa. A
lógica de "registrar o crash" foi extraída para uma função só, usada pelos dois
caminhos.

### 3. `WizardCreate` exigia todos os campos

**O que aconteceu.** Para criar um Postgres, o agente mandou o mínimo e recebeu,
um de cada vez, `missing field app_name`, `db_name`, `image`, `use_replica`,
`template_values`, `expose_external`. Desistiu e copiou o spec de outro serviço.

**Por quê.** `WizardCreateReq` não tinha `#[serde(default)]` em nada, embora o
`build_spec` **já** preencha tudo o que vem vazio (usuário padrão do motor,
banco `app`, imagem padrão, senha gerada). A GUI e a webui sempre mandam todos
os campos, então ninguém tinha notado.

**A correção.** `#[serde(default)]` em todos os campos opcionais. Continuam
obrigatórios só `kind`, `project_id` e `name` (`id` também fica com default,
porque `application`/`compose` não o usam). O manual ganha um exemplo mínimo
que funciona.

**Compatibilidade.** Só afrouxa a leitura: tudo que era aceito continua sendo,
com o mesmo resultado. Os clientes não mudam.

### 4. A ferramenta `rpc` recusava `"JobListAll"`

**O que aconteceu.** O manual diz que variante sem campos é "a string nua
(`"ProjectList"`)". Mandando `"JobListAll"`, a resposta foi
``unknown variant `"JobListAll"` `` (com as aspas dentro do nome). Sem as aspas
funcionou.

**Por quê.** Alguns clientes MCP entregam o argumento `command` como **texto
JSON** em vez de valor JSON: a string chega como `"\"JobListAll\""` (ou um
objeto chega como `"{\"ServiceGet\":…}"`). A ferramenta fazia `from_value`
direto, e o serde via um nome de variante com aspas.

**A correção.** Antes de interpretar, a ferramenta normaliza: se `command` é uma
string que, sem espaços nas pontas, começa com `"` ou `{`, ela é decodificada
como JSON (uma vez). Os dois formatos passam a funcionar: valor JSON e texto
JSON. O `input_schema` passa a dizer que `command` é string **ou** objeto.

### 5. O formato do healthcheck HTTP não estava no manual

**O que aconteceu.** O agente descobriu `{"Http":{"path":…,"expected_status":…}}`
pelo erro `missing field expected_status`.

**A correção.** Uma linha no manual, em "Serviços", com os quatro tipos de
healthcheck (`"None"`, `"Tcp"`, `{"Http":{path, expected_status}}`,
`"DockerNative"`) e os campos de tempo.

### 6. `JobCreate` recusava `trigger_service_id: null`

**O que aconteceu.** Encontrado durante o teste desta entrega: o `JobList` devolve
`"trigger_service_id": null` para um job sem gatilho, mas o `JobCreate` recusava
esse mesmo `null` (`invalid type: null, expected a string`) e só aceitava `""`.

**A correção.** O campo aceita `null`, chave ausente ou `""`, e os três significam
"sem gatilho", como já significavam no armazenamento.

## Como foi provado

- **Ponto 1, reproduzido nesta máquina** (daemon 0.1.74, Podman pelo socket): um
  job inline de uma linha (`busybox`, `echo migrations aplicadas; exit 0`) imprimiu a
  linha, o container saiu com 0, e o `docker compose up` continuou vivo por minutos,
  com o `job_run` aberto. Era o mesmo defeito do servidor, e não só do servidor.
- **Depois, num daemon de teste isolado com o código novo** (banco e portas próprios,
  o mesmo Podman), pela ferramenta MCP como um agente faria: o mesmo job terminou em
  56 s com sucesso e a linha do vigia no log; `"\"JobListAll\""` e um objeto em texto
  JSON passaram no `rpc`; o `WizardCreate` mínimo criou o Postgres; o `JobCreate`
  aceitou `trigger_service_id: null`; o container que morre ao subir falhou com
  `o container saiu com código 3 antes do healthcheck: Error: …` e o log dele no build
  log (o caminho novo, com o container já fora da rede); o deploy abortado durante o
  pré-deploy fechou o run com `-2` e a linha "Interrompida".
- **Pontos 3 e 4** reproduzidos no mesmo daemon antes da atualização
  (`missing field app_name`; ``unknown variant `"JobListAll"` ``).
- **Ponto 2**: no localhost o container ainda tinha IP quando o healthcheck
  começou, e o caminho antigo capturou o log. A corrida do servidor (container já
  fora da rede) não se reproduz aqui de forma determinística. O que muda e é
  conferível é a mensagem: antes `container stopped during healthcheck`, agora com
  o código de saída e a última linha.
- **0.1.77**, no mesmo daemon isolado: depois do abort, nenhum `docker-compose` órfão,
  nenhum container do job e nenhum diretório de build. Um teste unitário sobe um `sh`
  que dispara um `sleep` neto e confere que o guarda mata os dois.
- Testes unitários para o vigia (interpretação do `inspect`, carência, reinício),
  para o guarda do `job_run`, para a mensagem do container morto, para o
  `WizardCreateReq` mínimo, para a normalização do `rpc` e para o `JobCreate`.

## Os dois clientes

Nada aqui é tela nova. Os pontos 1 e 2 mudam o que aparece nos logs que GUI e
webui **já** mostram (log do job, build log do deploy). Os pontos 3, 4 e 5 são do
protocolo, da ferramenta do agente e do manual. Por isso o app desktop não precisa
de versão nova.

## Dados

Nenhuma mudança de schema, de YAML de infra ou de formato no fio. O `-2` do
`job_run` cancelado já existia. Uma versão anterior do daemon lê tudo o que esta
grava.

## O que fica para depois

- Registrar o `job_run` de pré-deploy em `active_jobs`, para que o
  `JobRunCancel` também cancele um pré-deploy (hoje o caminho é abortar o
  deploy, que agora fecha o run direito).
- Descobrir por que o compose do Podman não percebeu a saída. O vigia contorna
  o problema. Entender a causa exata exige reproduzir no servidor com Podman.
- Um `schemars` sobre `protocol`/`domain` para o manual mostrar o formato de
  qualquer tipo, em vez de exemplos curados à mão.
