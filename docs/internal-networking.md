# Comunicação interna: API Application ↔ Banco Compose

Este documento explica como um serviço do tipo **Application** (Registry ou Git) se comunica internamente com um serviço do tipo **Compose** (por exemplo, um banco de dados) dentro do Rustploy.

## O problema: duas redes separadas

No Rustploy, serviços Application e serviços Compose vivem em redes Docker diferentes por padrão:

- **Serviços Application** são conectados à rede compartilhada do projeto:
  - Nome: `rp_net_{primeiros_8_chars_do_project_id}` (ex.: `rp_net_a1b2c3d4`)
  - A conexão é feita pelo executor antes de iniciar o container.
  - O container live recebe o nome `rp_{service_name}`.

- **Serviços Compose** são executados com:

  ```
  docker compose -p rp_{service_name} -f - up -d --build --remove-orphans
  ```

  O Docker Compose cria automaticamente uma **rede própria**: `rp_{service_name}_default`. Os containers recebem nomes no padrão `rp_{service_name}-{nome_no_yaml}-1`.

  Exemplo: um serviço Compose chamado `mydb`, com um serviço `postgres` no YAML, gera o container `rp_mydb-postgres-1`.

Como essas duas redes são separadas, **a API não consegue resolver o nome do banco por padrão**. O Docker DNS só resolve nomes entre containers que compartilham a mesma rede user-defined.

## A solução

Para que a API alcance o banco, o compose file precisa **declarar a rede do projeto como externa** e **conectar o serviço do banco a ela**. Com ambos na mesma rede, o Docker DNS passa a resolver os nomes automaticamente.

São três passos.

### Passo 1 — Descobrir o ID do projeto

O nome da rede é `rp_net_` + os 8 primeiros caracteres do **Project ID**.

O Project ID aparece na aba **Projects** do TUI. Você também pode descobrir o nome da rede direto pelo Docker:

```bash
docker network ls | grep rp_net
```

Anote o nome completo da rede (ex.: `rp_net_a1b2c3d4`).

### Passo 2 — Escrever o compose file com a rede externa

Declare a rede do projeto como `external` e conecte o serviço do banco a ela:

```yaml
services:
  postgres:
    image: postgres:16
    environment:
      POSTGRES_PASSWORD: secret
      POSTGRES_DB: myapp
    networks:
      - rp_net          # alias local usado dentro deste compose file
    # volumes, etc.

networks:
  rp_net:
    external: true
    name: rp_net_a1b2c3d4   # substitua pelos 8 chars do SEU project ID
```

Pontos importantes:

- `external: true` diz ao Compose para **não criar** essa rede, e sim usar uma já existente (a rede do projeto, criada pelo Rustploy).
- O `name:` precisa bater exatamente com o nome real da rede do projeto.
- `rp_net` (sob `services` e como chave em `networks`) é apenas um **apelido local** dentro deste arquivo; não precisa ser o nome real da rede.

### Passo 3 — Configurar a env var da API

Aponte a URL de conexão da API para o nome do container do banco (próxima seção). Tipicamente isso vai em uma variável de ambiente do serviço Application, por exemplo:

```
DATABASE_URL=postgresql://user:password@rp_mydb-postgres-1:5432/myapp
```

## Nome do container e URL de conexão

Com ambos na mesma rede, a API resolve o banco pelo **nome do container**, seguindo o padrão:

```
rp_{nome_do_service_compose}-{nome_do_serviço_no_yaml}-1
```

Para um serviço Compose `mydb` com serviço `postgres` no YAML, o container é `rp_mydb-postgres-1`, e a URL fica:

```
postgresql://user:password@rp_mydb-postgres-1:5432/myapp
```

## Alternativa mais limpa: alias de rede

O nome `rp_mydb-postgres-1` é verboso e muda se você renomear o serviço Compose. Você pode definir um **alias de rede** estável no compose file:

```yaml
services:
  postgres:
    image: postgres:16
    networks:
      rp_net:
        aliases:
          - db   # a API pode usar "db" como hostname
```

Com isso, a API usa simplesmente `db` como hostname:

```
postgresql://user:password@db:5432/myapp
```

Essa é a abordagem recomendada: o hostname fica curto, legível e independente do nome interno do container.

## Exemplo completo

Cenário realista: um Postgres como banco (serviço Compose) e uma API Node ou FastAPI (serviço Application).

### Compose file do banco (serviço Compose `mydb`)

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
    networks:
      rp_net:
        aliases:
          - db

volumes:
  pgdata:

networks:
  rp_net:
    external: true
    name: rp_net_a1b2c3d4   # 8 chars do seu project ID
```

### Env var da API (serviço Application)

```
DATABASE_URL=postgresql://appuser:secret@db:5432/myapp
```

Node (exemplo com `pg`):

```js
import { Pool } from "pg";

const pool = new Pool({ connectionString: process.env.DATABASE_URL });
```

FastAPI / SQLAlchemy (exemplo):

```python
import os
from sqlalchemy import create_engine

engine = create_engine(os.environ["DATABASE_URL"])
```

Como ambos estão na rede `rp_net_a1b2c3d4`, o Docker DNS resolve `db` para o container do Postgres e a conexão funciona.

## Troubleshooting

**Verificar se o container do banco está na rede do projeto:**

```bash
docker network inspect rp_net_a1b2c3d4 \
  --format '{{range .Containers}}{{.Name}} {{end}}'
```

A saída deve listar tanto o container da API (`rp_{service_name}`) quanto o container do banco (`rp_mydb-postgres-1`). Se o banco não aparecer, o compose file não conectou o serviço à rede externa corretamente.

**Verificar a quais redes um container pertence:**

```bash
docker inspect rp_mydb-postgres-1 \
  --format '{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}'
```

**Testar resolução de DNS de dentro do container da API:**

```bash
docker exec rp_myapi getent hosts db
# ou, se houver ping disponível:
docker exec rp_myapi ping -c1 db
```

Se o nome não resolver:

- Confirme que o `name:` da rede externa no compose bate com o nome real (`docker network ls | grep rp_net`).
- Confirme que o serviço do banco tem o bloco `networks:` apontando para a rede externa.
- Lembre que a rede `rp_{service_name}_default`, criada pelo Compose, é separada e **não** dá acesso à API — a conexão precisa ser pela rede do projeto.
