# Limpeza automática do Docker: liberar espaço em disco sozinho, todos os dias

## O problema

Hoje o host acumula lixo do Docker: containers parados, imagens sem uso,
volumes órfãos, redes soltas e cache de build. Existe uma forma de limpar
isso — a aba **Docker** já tem botões "limpar" para cada tipo de recurso
(`crates/daemon/src/api/handlers/docker_prune.rs`) — mas é tudo manual. Se
ninguém entra na tela e clica, o disco vai enchendo até virar problema (build
falhando por falta de espaço, por exemplo).

O pedido é simples: um jeito de marcar "limpe isso sozinho, todo dia" e
esquecer do assunto.

## Por que não simplesmente usar a feature "Jobs" que já existe

O rustploy já tem uma engine de tarefas agendadas — a feature "Jobs"
(sidebar, antiga "Schedules"): você cadastra uma tarefa com um
`docker-compose`, ela sobe um stack efêmero e roda até terminar, com
agendamento por `Recurrence` (a cada N horas / todo dia às HH:MM / toda
semana num dia+hora). Dava pra pensar em usar exatamente isso: cadastrar um
Job cujo compose é basicamente `docker system prune`.

Não encaixa bem por um motivo estrutural: **todo Job pertence a um projeto**
e usa um **serviço "gatilho"** daquele projeto (pra emprestar rede Docker e
variáveis de ambiente) — é assim que o modelo de dados (`Job.project_id`,
`Job.main_service`) foi desenhado, porque a ideia original era "rodar uma
migration/teste ligado a um serviço". Limpeza de Docker não pertence a
projeto nenhum, é coisa do host inteiro (a própria aba Docker já lista
recursos do host todo, não só do que o rustploy criou). Forçar um projeto
"fake" só pra pendurar essa tarefa nele ia ser gambiarra visível na tela de
Jobs, e explicar pro usuário "ignore esse projeto, é só um truque" é mau
sinal de design.

**O que dá pra reaproveitar de verdade, e vale a pena reaproveitar:**
- O tipo `Recurrence` (IntervalHours / Daily / Weekly) já existe em
  `crates/shared/src/models.rs`, já tem o cálculo de "próxima execução"
  testado, e já tem um widget de formulário pronto (o seletor usado no
  cadastro de Job, em `new_job_window.luau`) — dá pra copiar o padrão sem
  reinventar nada.
- O padrão de "configuração única do daemon" (chave/valor em
  `daemon_settings`, usado hoje para e-mail do Let's Encrypt e domínio do
  registry) serve perfeitamente pra guardar essa configuração — não precisa
  de tabela nova no banco.
- As funções de prune que já existem (`docker_prune.rs`) continuam sendo a
  única implementação — o botão manual e a limpeza automática vão chamar
  exatamente o mesmo código Rust por baixo, só o gatilho muda.

Ou seja: a peça reaproveitada é o **agendamento** (Recurrence) e a
**execução** (prune handlers), não o modelo de dados de Job em si. É um
sistema pequeno e separado, do jeito que você pediu ("sessão separada, menu
separado") — e isso também evita misturar, na tela de Jobs, uma tarefa que
não é "um job do usuário" com as que são.

## Onde isso aparece na tela

Uma 4ª aba dentro de **Settings**, ao lado de "Web Server" / "Git" /
"Infra as Code": **"Manutenção"**. Fica junto das outras configurações do
daemon (faz sentido — é configuração de comportamento do daemon, não uma
ação pontual como os botões da aba Docker) mas em seção própria, sem
misturar com os botões manuais de limpar que já existem lá.

Conteúdo da aba:
- Um interruptor geral **"Ativar limpeza automática"**.
- O seletor de frequência (mesmo widget de Jobs): a cada N horas / todo dia
  às HH:MM / toda semana num dia+hora. Default sugerido: todo dia às 03:00.
- Uma checkbox por tipo de recurso, todas **desligadas por padrão** (é uma
  ação destrutiva rodando sozinha — o padrão seguro é pedir opt-in
  explícito, não limpar tudo no primeiro dia só porque a tela abriu):
  - Containers parados
  - Imagens sem uso (com uma segunda checkbox "incluir todas as não usadas,
    não só as sem tag" — o padrão do Docker só remove imagem "dangling"
    sem essa opção)
  - Cache de build
  - Redes sem uso
  - Volumes sem uso (com a mesma segunda checkbox "incluir todos") — este
    item leva um aviso visível de risco, ver seção seguinte.
- Botão **"Executar agora"**, pra testar sem esperar o horário — chama o
  mesmo caminho de código do agendador.
- Um resumo da última execução: quando rodou, quanto cada recurso liberou,
  e erros, se houver.

## O cuidado especial com Volumes

Volume não usado pode conter dado (backup de banco, etc.) que ninguém
recriaria automaticamente. Vale registrar uma observação tranquilizadora,
mas não uma isenção total de risco: **os serviços que o próprio rustploy
gerencia nunca usam volume nomeado** — o `CLAUDE.md` do projeto já documenta
isso ("rustploy never creates named volumes, only bind mounts"), então uma
limpeza de volumes não vai atingir nada que o rustploy tenha criado para os
seus serviços. O risco real é a volumes criados **por fora** (um
`docker volume create` manual, um Job com compose que declara volume
nomeado, outro stack não gerenciado pelo rustploy rodando no mesmo host).
Por isso a checkbox de volumes fica com o aviso mais forte da tela e exige
dois cliques (a geral + a "incluir todos") pra valer.

## Peças técnicas (pra quem for implementar)

**Config nova**, em `crates/shared/src/models.rs`:
```rust
pub struct DockerCleanupConfig {
    pub enabled: bool,
    pub recurrence: Recurrence,   // reaproveitado de Jobs
    pub containers: bool,
    pub images: bool,
    pub images_all: bool,
    pub volumes: bool,
    pub volumes_all: bool,
    pub networks: bool,
    pub build_cache: bool,
    pub last_run_at: Option<DateTime<Utc>>,
}
```
Guardada como um único JSON serializado, numa chave nova
(`docker_cleanup_config`) da tabela `daemon_settings` que já existe
(`crates/daemon/src/db/daemon_settings.rs`) — sem migração de schema.

**Protocolo** (`crates/shared/src/protocol.rs`):
- `Command::DockerCleanupConfigGet` / `DockerCleanupConfigSet` /
  `DockerCleanupRunNow`
- `Response::DockerCleanupConfig { config, last_result }`
- `Event::DockerCleanupCompleted { at, results, errors }` — pro
  toast/notificação aparecer nas duas telas (e na bandeja do sistema, que já
  tem esse fio ligado desde o glacier-ui 0.47).

**Execução**, módulo novo `crates/daemon/src/maintenance/`:
- `scheduler.rs` — ticker (mesmo idioma de `jobs::scheduler::scheduler_loop`
  e `metrics::collect_loop`, `tokio::spawn` a partir de `main.rs`), a cada
  tick carrega a config, se `enabled` calcula
  `recurrence.next_after(last_run_at)` e compara com agora.
- `run.rs` — roda os recursos marcados, um por um, chamando as MESMAS
  funções que os botões manuais chamam hoje.

**Pequeno refactor em `docker_prune.rs`**: hoje cada função
(`prune_containers`, `prune_images`, ...) devolve direto um
`shared::Response`. Vou extrair o miolo de cada uma pra uma função que
devolve um resultado tipado (contagem + bytes liberados, sem depender de
`Response`), e a função pública atual vira uma casca fina que só embrulha
isso em `Response` — assim o botão manual e o agendador chamam exatamente o
mesmo código, sem duplicar a chamada ao Docker.

**Tela**: `home.gv` (sub-aba nova em Settings, reaproveitando o CSS de
formulário que já existe) + `views/scripts/handlers/settings.luau` (ou
similar) na GUI iced; espelho em `crates/daemon/webui/screens/settings.js` +
`index.html` na web UI — os dois clientes falam o mesmo protocolo, então a
lógica é a mesma, só a marcação muda.

## Fora de escopo (por agora)

- **Agendamento por recurso** (ex.: imagens toda semana, containers todo
  dia): um agendamento único pra tudo que está marcado é mais simples de
  entender e cobre o pedido original ("todo dia"). Se um dia fizer falta,
  dá pra evoluir depois.
- **Histórico completo de execuções** (lista de todas as vezes que rodou,
  como existe para Jobs via `job_run`): por ora só guardo o resumo da
  última execução. Não crio tabela nova no banco só pra isso.
- **Rodar por projeto/serviço específico**: a limpeza é do host inteiro,
  igual à aba Docker já é hoje.
