# Comunicação interna entre serviços do projeto

No Rustploy, todos os serviços de um mesmo projeto se comunicam automaticamente pela rede Docker do projeto — sem nenhuma configuração manual necessária.

## Como funciona

Quando um projeto é criado, o Rustploy garante a existência de uma **rede bridge dedicada**:

```
rp_net_{id_do_projeto_inteiro_em_minúsculo}
```

O nome é **gravado** em `project.network_name` na criação e lido de lá em todo
uso (não é recalculado). Projetos criados antes de 2026-09-29 mantêm o nome
antigo, `rp_net_{8 primeiros chars do ID}` — a migração só o anotou. Ver
`docs/plano-nome-gravado-rede-e-stack.md`.

Todos os serviços do projeto — tanto **Application** (Registry/Git) quanto **Compose** — são conectados a essa rede automaticamente pelo daemon no momento do deploy.

### Serviços Application

O executor cria o container já na rede do projeto. O **nome do container** é
`rp_{id8}_{service_name}` (8 caracteres do ID do serviço — nome de container é
único no servidor inteiro, e dois projetos podem ter um serviço `api`). Quando
o deploy passa no healthcheck, o container ganha na rede do projeto o **alias**
`rp_{service_name}`, que é o nome que os outros serviços usam. Com réplicas,
todas recebem o mesmo alias e o DNS do Docker distribui entre elas.

(Até 2026-09-29 o nome do container era `rp_{service_name}`, sem ID. O
primeiro deploy depois da atualização troca o container; o hostname
`rp_{service_name}` continua o mesmo. Ver `docs/plano-colisao-nome-container.md`.)

### Serviços Compose

Antes de invocar `docker compose up`, o daemon injeta a rede do projeto no YAML do usuário (`docker/compose.rs → inject_project_network`). O compose file original **não precisa declarar nenhuma rede**; o Rustploy adiciona automaticamente:

- Um bloco `networks:` de topo com a rede do projeto como `external: true`
- A entrada dessa rede em todos os serviços do compose

O rewrite é idempotente: se a rede já estiver declarada no YAML, não é adicionada de novo.

## Renomear um serviço ou um projeto

Aba **General** do serviço → "Nome do serviço" (GUI e webui). O projeto se
renomeia em "Editar projeto".

| O que | Renomear projeto | Renomear serviço Compose | Renomear serviço Application |
|---|---|---|---|
| Rede Docker | não muda (gravada em `project.network_name`) | não muda | não muda |
| Stack e volumes | — | **não mudam** (stack gravada em `service.compose_project`) | — |
| Hostname interno | — | não muda: é a **chave do serviço no YAML** | `rp_{nome}` passa a valer no **próximo deploy**; o nome antigo deixa de resolver |
| Domínios, env vars, secrets | não mudam | não mudam | não mudam |

Só o serviço Application tem consequência para quem o chama: outros serviços que
usam `rp_{nome antigo}` em variáveis de ambiente precisam ser atualizados (e o
serviço, redeployado). A tela avisa disso ao lado do campo. O card "Internal URL"
de um serviço Compose mostra a chave do YAML (`ingress_service` se declarado,
senão a primeira de `services:`), não o nome do serviço.

Manifestos IaC casam projeto e serviço **por nome**: um manifesto antigo aplicado
depois de um rename cria um projeto/serviço novo em vez de atualizar o existente.

## Como referenciar outros serviços

### De um serviço Compose para outro serviço Compose (mesmo stack)

Use o nome do serviço conforme definido no YAML:

```yaml
services:

  api:
    image: minha-api
    environment:
      DB_URL: postgresql://user:pass@postgres:5432/mydb

  postgres:
    image: postgres:16
```

O Docker Compose resolve `postgres` internamente.

### De um serviço Application para um serviço Compose

Use o nome do container gerado pelo Rustploy:

```
rp_{id8}_{nome_do_service_compose}-{nome_do_serviço_no_yaml}-1
```

onde `{id8}` são 8 caracteres do ID do serviço. Exemplo: serviço Compose
`mydb` com serviço `postgres` no YAML → container
`rp_01jabcde_mydb-postgres-1`. O nome exato aparece no inventário Docker da
GUI/webui.

```
DATABASE_URL=postgresql://user:pass@rp_01jabcde_mydb-postgres-1:5432/mydb
```

A **chave do serviço no YAML** também resolve, como alias, dentro da rede do
projeto — no exemplo, `postgres`. Os bancos criados pelo wizard usam a chave
`rp_{nome}`, então `rp_mydb` funciona para eles. (Verificado em 2026-09-29: o
formato `rp_{nome}-{serviço}-1`, sem o ID, que este doc ensinava antes, **não
resolve**.)

### De um serviço Compose para um serviço Application

Use o nome do container do serviço Application:

```
rp_{nome_do_service}
```

Exemplo: serviço Application `myapi` → hostname `rp_myapi`.

## Exemplo completo

Dois serviços no mesmo projeto: `mydb` (Compose, Postgres) e `myapi` (Application, Node/FastAPI).

### Compose file do banco — sem configuração de rede necessária

```yaml
services:

  postgres:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_USER: appuser
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: myapp
    volumes:
      - pgdata:/var/lib/postgresql/data

volumes:
  pgdata:
```

O Rustploy injeta a rede do projeto automaticamente.

### Env var da API (serviço Application `myapi`)

```
DATABASE_URL=postgresql://appuser:secret@postgres:5432/myapp
```

## Troubleshooting

**Verificar se um container está na rede do projeto:**

```bash
docker network inspect <nome da rede do projeto> \
  --format '{{range .Containers}}{{.Name}} {{end}}'
```

A saída deve listar tanto o container da API (`rp_{id8}_myapi`) quanto os containers do banco (`rp_{id8}_mydb-postgres-1`).

**Testar resolução de DNS de dentro de um container:**

```bash
docker exec <container_da_api> getent hosts postgres
```

Se não resolver, confirme que o deploy do serviço Compose foi concluído com sucesso — a rede só é injetada no momento do `compose up`.
