# Backup dos dados dos serviços para S3, com restauração por uma tela de recuperação

> **Plano** (2026-11-11). **Estado: Fases 0 a 5 implementadas em 2026-11-11 (commitadas e publicadas no crates.io em 2026-11-11: shared 0.26.0, daemon 0.1.52, app 0.1.54). Falta ver as telas rodando e a recuperação com um remoto https real.** Origem: item "Backup de volumes para S3" do `TODO.md`, com as
> decisões do dono desta sessão: **a configuração do backup fica na aba Dados de cada serviço**, e a
> **restauração é uma tela de recuperação** (git + chave; o S3 só se o git não o trouxer) que recupera tudo sozinho. Vale para **todos os motores**.

## 1. O problema, em linguagem simples

Hoje a **configuração** do servidor já está protegida: o `state_git` empurra os YAMLs (cifrados) para um repositório
privado. Os **dados** não. Se a VPS some, você reconstrói projetos e serviços a partir do git, mas o banco, os
uploads e tudo que os containers gravaram se perdem. A exportação manual (aba Dados, `.rpdata`) resolve só se
alguém lembrar de clicar.

Meta: **a cada X horas o servidor copia os dados de cada serviço para um S3 qualquer** (AWS, Backblaze, R2, ou o
Storage do próprio Rustploy em outra máquina) e, num servidor novo e vazio, **uma tela com duas respostas**
(repositório git e chave de recuperação) traz tudo de volta, na ordem certa.

## 2. O que já existe e vamos reaproveitar

| Peça | Onde | Uso aqui |
|---|---|---|
| Gerador do pacote `.rpdata` (tar com manifesto e SHA-256 por parte) | `features/service_data/pack.rs`, `transfer::export` | é o conteúdo do backup, sem mudar de formato |
| Dump lógico de Postgres/MySQL/MariaDB/MongoDB | `service_data/database.rs` | motores de banco |
| Tar de volume em container descartável | `service_data/transfer.rs` | todos os outros motores (Redis, Kafka, RabbitMQ, NATS, RustFS/MinIO, apps) |
| Importação com rede de segurança (guarda o atual, devolve se falhar) | `transfer::import` | restauração de um serviço |
| Cifra `age` + chave de recuperação | `platform/secrets.rs` (`encrypt_file`, `has_recovery`) | nada sobe em claro |
| Restauração da infra com IDs preservados | `features/state_git` (`StateRestore`) | passo 1 da tela de recuperação |
| Ticker de agendamento | `features/jobs/scheduler.rs` | modelo do agendador de backups |

**O que não existe:** um cliente S3. O daemon só tem um GET HTTPS em hyper (`platform/https_client.rs`) e a regra
é **sem `reqwest`**. Vamos escrever um cliente S3 mínimo em hyper (assinatura SigV4 com `hmac` e `sha2`, que já
são dependências).

## 3. As decisões (com o porquê)

1. **O destino S3 é um só por servidor (configuração global); o *quando* e o *quanto guardar* são por serviço.**
   Endpoint, região, bucket, prefixo, estilo de endereço (path/virtual-host) e as chaves ficam em Settings →
   Backups. A aba Dados de cada serviço tem só: ligar, frequência, quantos manter. Assim a pessoa não digita o S3
   dez vezes, e a credencial existe em um lugar só.
2. **A chave de acesso do S3 é um secret cifrado** (`ENC[age:…]`), nunca em claro no `config.toml` nem no git.
   Como o destino mora no `project.yml`/infra e vai para o `state_git`, **depois de restaurar o servidor os
   backups continuam sozinhos**. (Ovo e galinha da chave `age`: a chave de recuperação já é pedida na tela de recuperação
   de qualquer jeito, então o destino cifrado dentro do git abre com ela. Mesmo assim a tela aceita digitar
   o S3 à mão, para o caso de o git não ter o destino.)
3. **Nada sobe em claro.** Ligar o backup exige `recovery_recipient` configurado. Sem ele, o backup abriria só
   no servidor que o gerou, e restaurar num servidor novo (a razão de existir do recurso) seria impossível.
4. **Um objeto por backup, nomeado por IDs que sobrevivem à restauração:**
   `<prefixo>/<project_id>/<service_id>/<AAAAMMDD-HHMMSS>.rpdata.age`. Os IDs entram no YAML e são preservados pelo
   `state_git`, então um servidor restaurado "enxerga" os backups do original. A lista de backups é o `LIST` do
   prefixo, sem banco de dados extra para dessincronizar.
5. **Retenção por contagem** (padrão: manter os últimos 7), apagada depois que o novo subiu com sucesso. Nunca
   apaga antes de ter o substituto.
6. **Todos os motores, pelo mesmo caminho do export:** banco com dump lógico (no ar), o resto com tar do volume
   (o serviço para durante a cópia, e a janela aparece na tela). Banco parado cai para o tar do volume (lacuna já
   anotada no item do export). Redis segue como volume por enquanto.
7. **Um backup por vez, e dentro da fila de deploy.** Como os jobs, o backup entra na mesma fila (uma coisa de
   cada vez por servidor), para não competir com um build por disco e CPU.
8. **Upload em partes (multipart)** acima de ~64 MiB, para volumes grandes não estourarem memória nem o limite de
   5 GiB de um PUT. Sem teto de tamanho, só a verificação de espaço em disco no plano (o pacote passa por disco
   antes de subir).

## 4. Onde fica na tela

**Aba Dados do serviço** (já existe, com exportar/importar):

- Bloco **Backup automático**: liga/desliga, frequência (de hora em hora / diário a uma hora / semanal), manter
  os últimos N, botão **Fazer backup agora**, e a **lista dos backups no S3** (data, tamanho) com **Restaurar este**
  (usa a importação atual, com a confirmação de sempre).
- Último backup: quando, quanto durou, tamanho, ou o erro.

**Settings → Backups** (novo): destino S3 + **Testar conexão** (grava, lê e apaga um objeto pequeno).

**Tela "Recuperar este servidor"** (nova; aparece num servidor vazio e em Settings). **Uma tela só, não um assistente
de vários passos** (decisão de 2026-11-11: como o destino S3 vai na infra, que o git já guarda, o passo do S3
deixa de existir no caso normal):

1. **Dois campos:** repositório git + token, e a chave de recuperação (`AGE-SECRET-KEY-…`).
2. **Resumo antes de agir:** o daemon clona, lê a infra e mostra projetos, serviços e quantos backups achou por
   serviço. A pessoa confirma.
3. **Execução com progresso:**
   - configura o `state_git` do servidor novo (URL, token, modo `sync`) e restaura a infra com os IDs
     preservados (`StateRestore`); o servidor segue sincronizando no mesmo repositório depois;
   - o destino S3 vem na infra (decisão 2) e a chave de acesso abre com a chave de recuperação;
   - para cada serviço, na ordem do `depends_on`, restaura o backup **mais recente** antes de subir; só onde o
     volume está vazio (nunca sobrescreve dado existente); depois sobe;
   - serviço sem backup sobe vazio e o resumo final avisa.
4. **O campo do S3 só aparece** se o git não trouxer o destino (configurado depois do último push). Nesse caso
   a pessoa o digita ali mesmo.

A lógica de dados mora no daemon; a tela é só o gatilho e o acompanhamento.

## 5. Modelo de dados e comandos

- `ServiceSpec.backup: Option<BackupPolicy { enabled, every_hours | daily_at, keep }>` (vai para o YAML e para o git).
- `BackupDestination` em Settings (secret cifrado para a chave).
- Comandos: `BackupDestinationGet/Set/Test`, `ServiceBackupNow`, `ServiceBackupList`, `ServiceBackupRestore`,
  `ServerRecoverPlan` / `ServerRecoverRun` (tela de recuperação). Níveis do agente: ler a lista e o plano é `observer`;
  configurar, rodar e restaurar são **`admin` e proibidos à chave de visitante** (um backup contém tudo que o
  agente nunca vê).
- Eventos de progresso pelo SSE de sempre; resultado do último backup gravado por serviço.

## 6. Fases

0. ✅ **Cliente S3** (`platform/s3.rs`; feito: 7 testes de unidade e teste `#[ignore]` contra RustFS em Podman, com multipart de 70 MiB): SigV4, PUT, multipart, GET, LIST, DELETE, HEAD; http e https; testes de
   unidade com os vetores oficiais da AWS e teste `#[ignore]` contra um RustFS local (Podman).
1. ✅ **Destino** (Settings → Backups; feito: `BackupDestinationGet/Set/Clear/Test`, `shared` 0.26.0 a publicar, chave cifrada em `settings.yml`, GUI e webui, nível `admin`, `Set`/`Test` proibidos ao agente; **não visto rodando** na GUI nem na webui): modelo, secret cifrado, `Test`, GUI e webui.
2. ✅ **Backup de um serviço** (feito; provado num daemon real com Podman rootless e RustFS: backup cifrado de volume com 3 MB, retenção `keep`, restauração com `md5` idêntico, recusa de chave de outro serviço e do agente; `ServiceBackupList/Now/Restore/Delete`, em segundo plano com `last_run`; bloco na aba Dados da GUI e da webui, **não visto rodando**; **bancos (dump) não foram exercitados neste fluxo**): `ServiceBackupNow` reaproveitando `transfer::export` (separar "gerar o arquivo" de
   "entregar o download") + upload + retenção + `ServiceBackupList` + `Restore`. Aba Dados, GUI e webui.
3. ✅ **Agendador** (feito; provado num daemon real: um `Daily` programado para o minuto seguinte disparou sozinho, gravou o backup e empurrou o próximo para o dia seguinte; sem destino o disparo falha uma vez, aparece na tela e não repete a cada minuto; política em `ServiceInfra.backup` no YAML, ligar exige destino + chave de recuperação; `ServiceBackupPolicySet`; formulário na aba Dados da GUI e da webui, **não visto rodando**; backup e restauração pegam a vaga da fila de deploy): ticker como o dos jobs; política por serviço; fila de deploy; "último backup".
4. ✅ **Tela de recuperação** (feita; **provada de ponta a ponta com dois daemons**: o servidor A, com um Postgres Compose e um serviço de volume, fez backup no S3 e empurrou a infra para um repositório git local; apagados os containers e volumes, o servidor B, virgem e sem chave de recuperação, trouxe a infra só com o repositório + a chave privada, achou o destino S3 e os 2 backups, restaurou o volume (`md5` idêntico, criado antes de existir container) e o banco (250 linhas, `md5` idêntico) e subiu na ordem das dependências).
   - **Decisão do dono (2026-11-11): a chave privada de recuperação é usada só na memória e depois descartada; os segredos são recifrados** para a identidade nova do servidor + a chave pública de recuperação (`store/infra/rekey.rs`, percorre o estado como JSON e recifra qualquer `ENC[age:…]`). Só a pública vai para o `config.toml`; verificado que a privada não está em nenhum arquivo do servidor novo.
   - Comandos: `ServerRecoverStart` (passo 1: infra + resumo, sem tocar em dado nem subir nada), `ServerRecoverRun` (passo 2, em segundo plano), `ServerRecoverStatus`, `ServerRecoverCancel`. `admin` e **proibidos ao agente**. A chave sai da memória ao terminar, ao cancelar ou após 1 hora parada.
   - Reaproveita o `orchestrate` do autostart com um `RecoveryBringer`: **volume restaura antes de subir** (cria o volume que ainda não existe, `parts::synthesize`; Compose usa `<stack>_<volume>`), **banco restaura depois** (o dump precisa dele no ar). Falhar ao consultar o S3 é falha do serviço (subir vazio e gravar por cima seria pior); sem destino ou sem backup, sobe vazio e o resumo avisa.
   - Só em servidor **limpo** (sem projetos): nunca sobrescreve.
   - Remoto aceito: `https://` (o token vira um provider git, validado no provedor), `file://` ou caminho absoluto (sem token).
   - **Não coberto pela prova:** um remoto `https://` real (GitHub/Gitea; só `file://` foi testado, o caminho do token usa o `GitProviderCreate` já existente); Docker (só Podman rootless); MySQL/MariaDB/MongoDB; as telas rodando (GUI e webui só têm testes automatizados).
   - **Consequências a conhecer:** (a) depois da recuperação, os backups **antigos** (cifrados para o servidor velho + recuperação) não abrem no servidor novo sem a chave: só os novos; (b) o provider "Recuperação" criado com o token fica cadastrado, e o `providers.yml` restaurado pode trazer o provider antigo, sem token (apague um deles); (c) um serviço que falha ao subir é tentado de novo (5, 15, 60, 180 e 300 s), então a fase fica "em andamento" por ~9 min antes de terminar como `failed`.
   - **Achado fora do escopo:** `ServiceSpec.volumes` com `host_path` sem `/` (um nome de volume) é lido como **volume nomeado** pelo export/backup e como **pasta do servidor** pelo deploy (falha com `statfs /…/nome`). Para serviço que não é Compose, só `host_path` absoluto funciona no deploy.
5. ✅ **Docs pt/en** (feitas; o site compila, 933 páginas e 0 links quebrados; **deploy não feito**): guias novos `guias/backup-no-s3` e `guias/recuperar-servidor` (pt e en, na barra lateral); ajustes em `backup`, `dados-do-servico`, `chaves-e-segredos`, `sincronizacao-git`, `interface/configuracoes` (agora sete abas, com Backups e Recuperar), `interface/servico` e `agentes/seguranca`; manual do agente e referência do protocolo regenerados (`npm run gen`). **Sem capturas de tela** (as abas novas nunca foram vistas rodando). O `CHANGELOG.md` registra releases publicadas: entra na hora do release.

## 7. Como testar (o teste que decide se está pronto)

- **Unidade:** SigV4 contra os vetores da AWS; montagem de chaves; retenção (qual apagar); política (quando vence);
  recusa de ligar sem `recovery_recipient`; ordem da recuperação.
- **Integração `#[ignore]` (Podman/Docker, RustFS local):** ida e volta de cada motor possível no ambiente
  (Postgres, volume comum, Redis…); falha de upload no meio não apaga o backup anterior; multipart.
- **Aceitação (regra do projeto, nada de dar por pronto sem rodar):** Postgres + app com volume → gravar dados →
  backup → **servidor limpo** → tela de recuperação → dados de volta. É o mesmo ambiente do teste de restauração do
  `state_git` do `TODO.md` (Docker/Podman como root): fazer os dois juntos.

## 8. Riscos e o que fica de fora

- **Consistência de volume comum** exige parar o serviço: a política mostra a janela e o padrão é fora do horário
  de pico escolhido pela pessoa.
- **Disco:** o pacote passa pelo disco local antes do upload (a verificação de espaço entra no plano).
- **Mongo:** só as coleções do pacote são substituídas (lacuna já conhecida do export).
- **Fora desta versão:** incremental/deduplicado (tar completo por execução), backup de volumes que não
  pertencem a serviço, notificação por e-mail/webhook em falha (a falha fica visível na tela e no evento),
  servidor→servidor direto.
- **Motores não testados em servidor real** hoje (MySQL, MariaDB, MongoDB, Docker) continuam não testados: o
  backup herda essa lacuna do export, e a aceitação deve cobri-los quando houver ambiente.
