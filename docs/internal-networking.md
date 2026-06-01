# Comunicação interna entre serviços do projeto

No Rustploy, todos os serviços de um mesmo projeto se comunicam automaticamente pela rede Docker do projeto — sem nenhuma configuração manual necessária.

## Como funciona

Quando um projeto é criado, o Rustploy garante a existência de uma **rede bridge dedicada**:

```
rp_net_{primeiros_8_chars_do_project_id}
```

Todos os serviços do projeto — tanto **Application** (Registry/Git) quanto **Compose** — são conectados a essa rede automaticamente pelo daemon no momento do deploy.

### Serviços Application

O executor conecta o container à rede do projeto via `docker network connect` antes do start, atribuindo o nome `rp_{service_name}` ao container.

### Serviços Compose

Antes de invocar `docker compose up`, o daemon injeta a rede do projeto no YAML do usuário (`docker/compose.rs → inject_project_network`). O compose file original **não precisa declarar nenhuma rede**; o Rustploy adiciona automaticamente:

- Um bloco `networks:` de topo com a rede do projeto como `external: true`
- A entrada dessa rede em todos os serviços do compose

O rewrite é idempotente: se a rede já estiver declarada no YAML, não é adicionada de novo.

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
rp_{nome_do_service_compose}-{nome_do_serviço_no_yaml}-1
```

Exemplo: serviço Compose `mydb` com serviço `postgres` no YAML → container `rp_mydb-postgres-1`.

```
DATABASE_URL=postgresql://user:pass@rp_mydb-postgres-1:5432/mydb
```

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
DATABASE_URL=postgresql://appuser:secret@rp_mydb-postgres-1:5432/myapp
```

## Troubleshooting

**Verificar se um container está na rede do projeto:**

```bash
docker network inspect rp_net_<8chars> \
  --format '{{range .Containers}}{{.Name}} {{end}}'
```

A saída deve listar tanto o container da API (`rp_myapi`) quanto os containers do banco (`rp_mydb-postgres-1`).

**Testar resolução de DNS de dentro de um container:**

```bash
docker exec rp_myapi getent hosts rp_mydb-postgres-1
```

Se não resolver, confirme que o deploy do serviço Compose foi concluído com sucesso — a rede só é injetada no momento do `compose up`.
