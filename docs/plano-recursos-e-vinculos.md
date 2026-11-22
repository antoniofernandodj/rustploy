# Plano: vínculos entre serviços (e confirmação em toda ação destrutiva)

> **Status (2026-11-22):** Fase 0, Fase 1 e Confirmações **publicadas** na v0.1.86 (daemon 0.1.71, app
> 0.1.79). **Fase 2 implementada** nesta entrega (§7; daemon 0.1.72, app 0.1.80). Fase 3 **não começou**.
> Decisões do dono em §2; as que tomei sem ele, na entrega autônoma da Fase 2, estão marcadas em §7.4.

## 1. O problema, em termos simples

Um serviço quase nunca roda sozinho: a API usa o banco, o S3, a fila e o cache. Hoje o Rustploy
sabe disso só pela metade:

- **A ordem** existe (`depends_on`): o `api` sobe depois do `db`.
- **A ligação** não existe: o `api` só alcança o S3 porque alguém digitou à mão, nas variáveis do
  projeto, `S3_ENDPOINT=http://rp_s3:9000`, `POSTGRES_HOST=rp_db`, `REDIS_HOST=rp_cache`… No
  Chiquitos são ~12 variáveis desse tipo. Renomear um serviço ou trocar uma porta não avisa ninguém.
- **O banco compartilhado** é configurado **do lado do banco**: na tela do servidor de banco você diz
  "o projeto P pode me usar". A informação mora no lugar errado para quem está olhando a API, e é
  por **projeto**, não por serviço.
- **O preparo** de um recurso (criar o bucket, o usuário do banco) virou um job genérico pendurado
  num pré-deploy check. Foi isso que derrubou o Chiquitos em 2026-11-20: o job que cria o bucket
  estava no próprio S3, que ainda não tinha subido (ver `docs/release.md`, v0.1.85).

## 2. O que o dono decidiu

1. **Quem declara é quem usa.** Abro o `api` e digo "uso o `db`", "uso o `s3`". É o mesmo sentido do
   `depends_on` e do `package.json`.
2. **Mas dá para editar dos dois lados.** O que existe hoje continua (o banco compartilhado segue
   aceitando projetos pela tela dele), e a tela de cada serviço ganha a edição do lado de quem usa.
   Na tela de quem oferece aparece "quem usa este serviço", **também editável** (adicionar ou tirar
   um consumidor grava no serviço do consumidor).
3. **Trava opcional** no provedor: "aceita vínculos de outros projetos". Não é segurança (o Rustploy
   tem um administrador só), é proteção contra engano.
4. **Toda ação destrutiva pede confirmação. Toda.** Em 2026-11-21 o dono clicou em "Esquecer este
   servidor" por engano no app, e o servidor salvo sumiu sem pergunta.
5. **Atualizar por comando** o app desktop e o daemon desta máquina, sem senha e sem clicar.

## 3. O modelo

### 3.1 Vínculo

Um campo novo no serviço que **usa**:

```yaml
# services/api.yml
links:
  - service_id: svc_01M6GVWREEVG10460NC4HHBXEY   # o s3
    env_prefix: S3                               # opcional; padrão = nome do serviço em maiúsculas
  - service_id: svc_01M6EMRN1MDTARE52AQEMQVHRX   # o db
```

O que um vínculo faz, sozinho:

- **Ordem:** conta como dependência. O `DeployWithDependencies`, o religamento no boot e a checagem de
  ciclo passam a enxergá-lo, sem precisar repetir o provedor em `depends_on`.
- **Variáveis:** no deploy, o consumidor recebe `<PREFIXO>_HOST`, `<PREFIXO>_PORT` e, quando o
  provedor é um banco ou S3 conhecido, `<PREFIXO>_URL` (a mesma URL interna da aba Conexão). **As
  variáveis que você digitou continuam valendo e ganham**: o vínculo só preenche o que falta. Assim o
  Chiquitos segue igual no dia da atualização, e as variáveis manuais podem ser apagadas aos poucos.
- **Rede entre projetos:** vincular um serviço a um **banco compartilhado** de outro projeto concede o
  acesso do projeto do consumidor (o mesmo `shared_access` de hoje), se ele ainda não existe.
  Desfazer o vínculo **não** revoga o acesso (o acesso pode ter sido dado à mão; revogar continua
  sendo pela tela do banco, com confirmação).

### 3.2 Quem pode ser vinculado

- Qualquer serviço **do mesmo projeto**.
- De **outro projeto**, só um **banco compartilhado** (é o único jeito de uma rede alcançar a outra), e
  só se ele não estiver com a trava ligada.

### 3.3 A trava

`shared.accept_links` no serviço do banco compartilhado (padrão **ligada = aceita**, porque um servidor
compartilhado existe para ser usado). Desligada, recusa **vínculos novos** de outros projetos; os que já
existem continuam funcionando.

### 3.4 Confirmação em toda ação destrutiva

Regra para os dois clientes (app e web UI): apagar, remover, esquecer, parar, limpar, revogar,
sobrescrever, reverter e desvincular sempre perguntam antes, dizendo **o que** some. A lista de ações
conferidas está em §6.

## 4. Migração dos dados

- **Nada precisa ser convertido.** `links` e `accept_links` são campos novos com valor padrão; um YAML
  antigo é lido como "sem vínculos, trava aceitando". O esquema do YAML de infra continua o 2.
- O `shared_access` de hoje continua do jeito que está, e o vínculo convive com ele.
- **Voltar para uma versão anterior:** o daemon antigo lê o YAML novo e ignora os campos novos. Mas se
  ele regravar um serviço (qualquer edição), o `links` daquele serviço se perde. As variáveis
  injetadas param de vir, e as que você digitou continuam. Está dito nas notas da versão.

## 5. As fases

| Fase | O quê | Estado |
|---|---|---|
| **0** | `ApplyAppUpdate`: o helper root do daemon instala o `rustploy-gui` assinado desta máquina, por comando, sem senha | **feita** |
| **1** | vínculos declarados no consumidor, editáveis dos dois lados, trava, ordem e variáveis deduzidas, acesso entre projetos | **feita** |
| **C** | confirmação em toda ação destrutiva, nos dois clientes | **feita** |
| 2 | **recursos provisionados**: o vínculo diz "o bucket `uploads`", "o banco `app`", e o Rustploy cria no provedor depois que ele sobe e antes do consumidor (substitui jobs como o `bucket` do Chiquitos) | **feita** (§7) |
| 3 | credencial **por vínculo** (cada serviço com o próprio usuário no banco); exportar/importar levando os vínculos; converter os acessos por projeto em vínculos por serviço | não começou |

### 5.1 Atualizar por comando (Fase 0)

```bash
# o daemon desta máquina (o token é o da API, [api].token)
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:9797/api/rpc -d '{"ApplyUpdate":{}}'
# o app desktop desta máquina, depois que o daemon já estiver numa versão com o ApplyAppUpdate
curl -s -H "Authorization: Bearer $TOKEN" http://127.0.0.1:9797/api/rpc -d '"ApplyAppUpdate"'
```

O helper só instala pacotes **assinados** pela chave do projeto, nunca uma versão menor, e o
`ApplyAppUpdate` só aceita o pacote `rustploy-gui` (o `install` segue aceitando só o `rustployd`). O app
aberto continua na versão antiga até ser reaberto. Pelo agente MCP vale a mesma trava do
`ApplyUpdate` (`[update] allow_agent_apply`).

### 3.5 O que a Fase 1 não cobre (de propósito)

- **Serviço Compose como consumidor:** as variáveis do vínculo vão para o `.env` da stack, como
  qualquer variável; só chegam ao container se o YAML as referenciar (`${DB_URL}`).
- ~~Jobs não recebem as variáveis dos vínculos~~ **(correção, 2026-11-22):** recebem desde a v0.1.86,
  porque o `resolve_job` usa o `resolve()` do serviço gatilho, que já inclui os vínculos. O texto
  original desta seção e o aviso nas docs estavam errados; ver §7.3.
- **Banco compartilhado de outro projeto:** a `_URL` é a do database gerenciado do projeto
  consumidor nesse servidor, se houver exatamente um; nunca a do superusuário do servidor. Sem
  database gerenciado, vão só `_HOST` e `_PORT`.
- Criar o database gerenciado continua na tela do banco (Fase 3 leva isso para o vínculo).

## 6. Ações destrutivas conferidas

Levantadas por varredura dos handlers dos dois clientes (todo comando destrutivo do protocolo e
toda função cujo nome remove, limpa, esquece, revoga, cancela ou para), depois conferidas uma a uma.
Ações que só cancelam um formulário ou limpam estado local não entram.

| Ação | App | Web UI |
|---|---|---|
| Esquecer o servidor salvo (login) | **passou a confirmar** (o caso do dono) | não existe |
| Cancelar deploy na fila / em curso | **passou a confirmar** | **passou a confirmar** |
| Parar container (Docker) | **passou a confirmar** | **passou a confirmar** |
| Limpar containers, imagens, volumes, redes | já confirmava | **passou a confirmar** |
| Remover container, imagem, volume, rede | já confirmava | **passou a confirmar** |
| Registry: remover tag, remover repositório, revogar token | já confirmava | **passou a confirmar** |
| Cancelar execução de job | **passou a confirmar** | **passou a confirmar** |
| Cancelar recuperação do servidor | **passou a confirmar** | **passou a confirmar** |
| Sync com git: "Esquecer" uma remoção | **passou a confirmar** | **passou a confirmar** |
| Fechar a chave de recuperação recém-gerada | **passou a confirmar** | **passou a confirmar** |
| Remover domínio, porta TCP, dependência, check da fila | **passou a confirmar** | **passou a confirmar** |
| Remover conta git sem uso (com uso já confirmava) | **passou a confirmar** | **passou a confirmar** |
| Apagar deployment | já confirmava | **passou a confirmar** |
| Desfazer vínculo (dos dois lados) | novo, confirma | novo, confirma |
| Apagar projeto, serviço, secret, variável, job, backup, log, banco gerenciado; parar tudo; reverter atualização; regenerar webhook; restaurar backup; migração | já confirmavam | já confirmavam |

## 7. Fase 2: recursos provisionados

### 7.1 O problema

No Chiquitos, o bucket `chickie-uploads` é criado por um **job** (`bucket`), pendurado como pré-deploy
check. Isso funciona, mas é frágil: o job precisa de um repositório, de um build, de variáveis
digitadas à mão para achar o S3 e, como se viu em 2026-11-20, alguém pode pendurá-lo no lugar errado.
Criar um bucket ou um database é uma coisa que o Rustploy sabe fazer sozinho, desde que saiba **qual**.

### 7.2 O modelo

O vínculo ganha um **recurso** opcional: o nome de uma coisa que o provedor guarda para o consumidor.

```yaml
links:
  - service_id: svc_s3
    resource: { name: chickie-uploads, public_read: true }   # um bucket
  - service_id: svc_db
    resource: { name: loja }                                  # um database
```

| Provedor (`product`) | O recurso é | Criado com | Variável a mais |
|---|---|---|---|
| `rustfs`, `minio` | um **bucket** | a API S3 do próprio provedor (as credenciais dele) | `<PREFIXO>_BUCKET` |
| `postgres`, `mysql`, `mariadb` | um **database** | SQL dentro do container do provedor (o usuário dele) | `<PREFIXO>_DATABASE`, e a `_URL` passa a apontar para ele |

- **Quando:** no deploy do consumidor, **antes** dos pré-deploy checks. Assim um job de migração já
  encontra o database, e o deploy em árvore garante que o provedor subiu antes.
- **Idempotente:** se já existe, não faz nada (só registra "já existia" no log do deploy).
- **Falhou?** O deploy para antes de mexer em qualquer coisa, com a causa no log; o que estava no ar
  continua (inclusive num consumidor Compose, desde a v0.1.85).
- **Leitura pública** (`public_read`, só bucket): aplica a política que deixa qualquer um **ler** os
  objetos (o que o job do Chiquitos faz hoje para servir as imagens).
- **Desfazer o vínculo não apaga o recurso.** Apagar dados nunca é efeito colateral.

### 7.3 Jobs recebem as variáveis dos vínculos

Um job com serviço gatilho (inclusive os de pré-deploy) recebe as variáveis dos vínculos desse serviço,
com a mesma menor precedência: a migração do `api` enxerga `DB_URL` sem cópia manual. Isso já valia
desde a v0.1.86 (o `resolve_job` reaproveita o `resolve()` do serviço); a Fase 2 só corrigiu a
documentação, que dizia o contrário.

### 7.5 Verificação e o que mudou no caminho

- Ponta a ponta num daemon isolado com Podman, rodando no namespace de rede rootless
  (`podman unshare --rootless-netns`, onde os IPs dos containers são alcançáveis como num host rootful):
  1º deploy criou o bucket (com leitura pública) no RustFS e o database no Postgres; 2º deploy disse
  "já existia"; o container do consumidor recebeu `S3_BUCKET`, `DB_DATABASE` e `DB_URL` apontando para o
  database; acesso anônimo a um objeto do bucket deu 404 (sem a política, 403).
- O primeiro roteiro, fora do namespace, mostrou um defeito: com o provedor inalcançável, o deploy
  ficava pendurado até 15 minutos (o cliente S3 do backup tenta 3 vezes com 300 s). O provisionamento
  ganhou um teto de 60 s, com a causa no log.

### 7.4 Decisões tomadas sem o dono (entrega autônoma)

1. **Só no mesmo projeto.** Recurso num banco compartilhado de outro projeto é recusado: para isso já
   existe o database gerenciado (com usuário próprio), e duas formas de fazer a mesma coisa confundem.
2. **Database sem usuário próprio:** é dono o usuário do provedor. Usuário por vínculo é a Fase 3.
3. **MongoDB e RabbitMQ (vhost) ficam de fora:** o Mongo cria database sozinho no primeiro uso, e o
   vhost do RabbitMQ não tinha caso real pedindo.
4. **O Chiquitos não foi alterado:** produção só leitura nesta entrega. Para trocar o job `bucket` por um
   recurso: no `api`, vincular ao `s3` com recurso `chickie-uploads` e leitura pública, fazer um deploy,
   e só então tirar o job da fila de pré-deploy.
5. **Migração de dados:** nenhuma; `resource` é campo novo e opcional. Um daemon antigo ignora o campo
   (e o perde se regravar o serviço), como na Fase 1.
