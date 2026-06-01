use super::{Template, TemplateCategory, TemplateVar};

// ─────────────────────────────────────────────────────────────────────────────
// To add a new template: append a `Template { .. }` entry to TEMPLATES below.
// The compose string uses {{KEY}} placeholders matched by variables[*].key.
// ─────────────────────────────────────────────────────────────────────────────

pub static TEMPLATES: &[Template] = &[
    // ── CMS ──────────────────────────────────────────────────────────────────

    Template {
        id: "wordpress",
        name: "WordPress",
        description: "CMS mais usado do mundo (MySQL)",
        category: TemplateCategory::Cms,
        default_port: 80,
        compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wordpress
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql

  wordpress:
    image: wordpress:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wordpress
      WORDPRESS_DB_PASSWORD: {{DB_PASSWORD}}
      WORDPRESS_DB_NAME: wordpress
    volumes:
      - wp_data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  wp_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar {
                key: "DB_PASSWORD",
                label: "Senha do banco",
                default: None,
                required: true,
                secret: true
              },
        ],
    },

    Template {
        id: "ghost",
        name: "Ghost",
        description: "Plataforma de blog e newsletter profissional",
        category: TemplateCategory::Cms,
        default_port: 2368,
        compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: ghost
    volumes:
      - db_data:/var/lib/mysql

  ghost:
    image: ghost:latest
    restart: unless-stopped
    ports:
      - "2368"
    environment:
      database__client: mysql
      database__connection__host: db
      database__connection__database: ghost
      database__connection__user: root
      database__connection__password: {{DB_ROOT_PASSWORD}}
      url: http://{{DOMAIN}}
    volumes:
      - ghost_data:/var/lib/ghost/content
    depends_on:
      - db

volumes:
  db_data:
  ghost_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DOMAIN",           label: "Domínio (ex: meusite.com)", default: Some("localhost:2368"), required: true, secret: false },
        ],
    },

    Template {
        id: "classicpress",
        name: "ClassicPress",
        description: "Fork estável do WordPress com editor clássico",
        category: TemplateCategory::Cms,
        default_port: 80,
        compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: classicpress
      MYSQL_USER: classicpress
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql

  classicpress:
    image: wordpress:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: classicpress
      WORDPRESS_DB_PASSWORD: {{DB_PASSWORD}}
      WORDPRESS_DB_NAME: classicpress
    volumes:
      - cp_data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  cp_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD",      label: "Senha do banco",   default: None, required: true, secret: true },
        ],
    },

    // ── Analytics ─────────────────────────────────────────────────────────────

    Template {
        id: "plausible",
        name: "Plausible",
        description: "Analytics focado em privacidade (Postgres + Clickhouse)",
        category: TemplateCategory::Analytics,
        default_port: 8000,
        compose: r#"
services:
  plausible_db:
    image: postgres:14
    restart: unless-stopped
    environment:
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
      POSTGRES_DB: plausible
    volumes:
      - db_data:/var/lib/postgresql/data

  plausible_events_db:
    image: clickhouse/clickhouse-server:latest
    restart: unless-stopped
    volumes:
      - event_data:/var/lib/clickhouse

  plausible:
    image: ghcr.io/plausible/community-edition:v2
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      BASE_URL: http://{{DOMAIN}}
      SECRET_KEY_BASE: {{SECRET_KEY_BASE}}
      DATABASE_URL: postgres://postgres:{{DB_PASSWORD}}@plausible_db:5432/plausible
      CLICKHOUSE_DATABASE_URL: http://plausible_events_db:8123/plausible_events
    depends_on:
      - plausible_db
      - plausible_events_db

volumes:
  db_data:
  event_data:
"#,
        variables: &[
            TemplateVar { key: "DOMAIN",          label: "Domínio base",        default: Some("localhost:8000"), required: true,  secret: false },
            TemplateVar { key: "DB_PASSWORD",     label: "Senha do banco",       default: None,                  required: true,  secret: true  },
            TemplateVar { key: "SECRET_KEY_BASE", label: "Secret Key (64 chars)", default: None,                 required: true,  secret: true  },
        ],
    },

    Template {
        id: "umami",
        name: "Umami",
        description: "Analytics web leve, alternativa ao Google Analytics",
        category: TemplateCategory::Analytics,
        default_port: 3000,
        compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: umami
      POSTGRES_USER: umami
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  umami:
    image: ghcr.io/umami-software/umami:postgresql-latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://umami:{{DB_PASSWORD}}@db:5432/umami
      DATABASE_TYPE: postgresql
      APP_SECRET: {{APP_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_SECRET",  label: "App Secret",     default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "ackee",
        name: "Ackee",
        description: "Analytics focado em privacidade para websites",
        category: TemplateCategory::Analytics,
        default_port: 3000,
        compose: r#"
services:
  db:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - db_data:/data/db

  ackee:
    image: electerious/ackee:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      MONGODB: mongodb://db:27017/ackee
      ACKEE_USERNAME: {{ADMIN_USER}}
      ACKEE_PASSWORD: {{ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_USER",     label: "Usuário admin", default: Some("admin"), required: true,  secret: false },
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin",   default: None,          required: true,  secret: true  },
        ],
    },

    // ── Monitoring ────────────────────────────────────────────────────────────

    Template {
        id: "uptime-kuma",
        name: "Uptime Kuma",
        description: "Monitor visual de uptime com alertas",
        category: TemplateCategory::Monitoring,
        default_port: 3001,
        compose: r#"
services:
  uptime-kuma:
    image: louislam/uptime-kuma:latest
    restart: unless-stopped
    ports:
      - "3001"
    volumes:
      - uptime_data:/app/data

volumes:
  uptime_data:
"#,
        variables: &[],
    },

    Template {
        id: "grafana",
        name: "Grafana",
        description: "Dashboards de observabilidade e métricas",
        category: TemplateCategory::Monitoring,
        default_port: 3000,
        compose: r#"
services:
  grafana:
    image: grafana/grafana:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      GF_SECURITY_ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - grafana_data:/var/lib/grafana

volumes:
  grafana_data:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "dozzle",
        name: "Dozzle",
        description: "Visualizador em tempo real de logs Docker",
        category: TemplateCategory::Monitoring,
        default_port: 8080,
        compose: r#"
services:
  dozzle:
    image: amir20/dozzle:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
"#,
        variables: &[],
    },

    // ── DevTools ──────────────────────────────────────────────────────────────

    Template {
        id: "gitea-sqlite",
        name: "Gitea",
        description: "Servidor Git leve e auto-hospedado (SQLite)",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"
services:
  gitea:
    image: gitea/gitea:latest
    restart: unless-stopped
    ports:
      - "3000"
      - "22"
    environment:
      USER_UID: 1000
      USER_GID: 1000
    volumes:
      - gitea_data:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro

volumes:
  gitea_data:
"#,
        variables: &[],
    },

    Template {
        id: "portainer",
        name: "Portainer",
        description: "Painel visual para gerenciamento de containers Docker",
        category: TemplateCategory::DevTools,
        default_port: 9000,
        compose: r#"
services:
  portainer:
    image: portainer/portainer-ce:latest
    restart: unless-stopped
    ports:
      - "9000"
      - "9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data

volumes:
  portainer_data:
"#,
        variables: &[],
    },

    Template {
        id: "filebrowser",
        name: "FileBrowser",
        description: "Gerenciador de arquivos web com controle de usuários",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"
services:
  filebrowser:
    image: filebrowser/filebrowser:latest
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - fb_data:/database
      - fb_files:/srv

volumes:
  fb_data:
  fb_files:
"#,
        variables: &[],
    },

    Template {
        id: "meilisearch",
        name: "Meilisearch",
        description: "Motor de busca textual open-source extremamente rápido",
        category: TemplateCategory::DevTools,
        default_port: 7700,
        compose: r#"
services:
  meilisearch:
    image: getmeili/meilisearch:latest
    restart: unless-stopped
    ports:
      - "7700"
    environment:
      MEILI_MASTER_KEY: {{MASTER_KEY}}
    volumes:
      - meili_data:/meili_data

volumes:
  meili_data:
"#,
        variables: &[
            TemplateVar { key: "MASTER_KEY", label: "Master Key", default: None, required: true, secret: true },
        ],
    },

    // ── Communication ─────────────────────────────────────────────────────────

    Template {
        id: "mattermost",
        name: "Mattermost",
        description: "Chat corporativo focado em DevOps (Postgres)",
        category: TemplateCategory::Communication,
        default_port: 8065,
        compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: mattermost
      POSTGRES_USER: mattermost
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  mattermost:
    image: mattermost/mattermost-team-edition:latest
    restart: unless-stopped
    ports:
      - "8065"
    environment:
      MM_SQLSETTINGS_DRIVERNAME: postgres
      MM_SQLSETTINGS_DATASOURCE: postgres://mattermost:{{DB_PASSWORD}}@db:5432/mattermost?sslmode=disable
      MM_SERVICESETTINGS_SITEURL: http://{{DOMAIN}}
    volumes:
      - mm_data:/mattermost/data
    depends_on:
      - db

volumes:
  db_data:
  mm_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None,                   required: true, secret: true  },
            TemplateVar { key: "DOMAIN",      label: "URL do site",    default: Some("localhost:8065"), required: true, secret: false },
        ],
    },

    Template {
        id: "rocketchat",
        name: "Rocket.Chat",
        description: "Ecossistema completo de chat e comunicação corporativa",
        category: TemplateCategory::Communication,
        default_port: 3000,
        compose: r#"
services:
  mongo:
    image: mongo:6
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - mongo_data:/data/db

  mongo-init-replica:
    image: mongo:6
    command: >
      bash -c "sleep 5 && mongosh --host mongo:27017 --eval \"rs.initiate({_id:'rs0',members:[{_id:0,host:'mongo:27017'}]})\""
    depends_on:
      - mongo

  rocketchat:
    image: registry.rocket.chat/rocketchat/rocket.chat:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      MONGO_URL: mongodb://mongo:27017/rocketchat?replicaSet=rs0
      MONGO_OPLOG_URL: mongodb://mongo:27017/local?replicaSet=rs0
      ROOT_URL: http://{{DOMAIN}}
      PORT: 3000
    depends_on:
      - mongo

volumes:
  mongo_data:
"#,
        variables: &[
            TemplateVar { key: "DOMAIN", label: "URL do site", default: Some("localhost:3000"), required: true, secret: false },
        ],
    },

    // ── Storage ───────────────────────────────────────────────────────────────

    Template {
        id: "nextcloud",
        name: "Nextcloud",
        description: "Suite completa de produtividade na nuvem (Postgres)",
        category: TemplateCategory::Storage,
        default_port: 80,
        compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: nextcloud
      POSTGRES_USER: nextcloud
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  nextcloud:
    image: nextcloud:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      POSTGRES_HOST: db
      POSTGRES_DB: nextcloud
      POSTGRES_USER: nextcloud
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
      NEXTCLOUD_ADMIN_USER: {{ADMIN_USER}}
      NEXTCLOUD_ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - nc_data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  nc_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD",     label: "Senha do banco", default: None,          required: true, secret: true  },
            TemplateVar { key: "ADMIN_USER",      label: "Usuário admin",  default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "ADMIN_PASSWORD",  label: "Senha admin",    default: None,          required: true, secret: true  },
        ],
    },

    Template {
        id: "minio",
        name: "MinIO",
        description: "Armazenamento de objetos compatível com S3",
        category: TemplateCategory::Storage,
        default_port: 9000,
        compose: r#"
services:
  minio:
    image: minio/minio:latest
    restart: unless-stopped
    ports:
      - "9000"
      - "9001"
    environment:
      MINIO_ROOT_USER: {{ROOT_USER}}
      MINIO_ROOT_PASSWORD: {{ROOT_PASSWORD}}
    command: server /data --console-address ":9001"
    volumes:
      - minio_data:/data

volumes:
  minio_data:
"#,
        variables: &[
            TemplateVar { key: "ROOT_USER",     label: "Usuário root", default: Some("minioadmin"), required: true, secret: false },
            TemplateVar { key: "ROOT_PASSWORD", label: "Senha root",   default: None,               required: true, secret: true  },
        ],
    },

    // ── Security ─────────────────────────────────────────────────────────────

    Template {
        id: "vaultwarden",
        name: "Vaultwarden",
        description: "Gerenciador de senhas compatível com Bitwarden",
        category: TemplateCategory::Security,
        default_port: 80,
        compose: r#"
services:
  vaultwarden:
    image: vaultwarden/server:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      ADMIN_TOKEN: {{ADMIN_TOKEN}}
    volumes:
      - vw_data:/data

volumes:
  vw_data:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_TOKEN", label: "Token admin (argon2 hash recomendado)", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "authentik",
        name: "Authentik",
        description: "Gerenciador de identidade e SSO (OAuth2, SAML, OIDC)",
        category: TemplateCategory::Security,
        default_port: 9000,
        compose: r#"
services:
  postgresql:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_DB: authentik
      POSTGRES_USER: authentik
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  redis:
    image: redis:alpine
    restart: unless-stopped

  server:
    image: ghcr.io/goauthentik/server:latest
    restart: unless-stopped
    command: server
    ports:
      - "9000"
      - "9443"
    environment:
      AUTHENTIK_REDIS__HOST: redis
      AUTHENTIK_POSTGRESQL__HOST: postgresql
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: {{DB_PASSWORD}}
      AUTHENTIK_POSTGRESQL__NAME: authentik
      AUTHENTIK_SECRET_KEY: {{SECRET_KEY}}
    depends_on:
      - postgresql
      - redis

  worker:
    image: ghcr.io/goauthentik/server:latest
    restart: unless-stopped
    command: worker
    environment:
      AUTHENTIK_REDIS__HOST: redis
      AUTHENTIK_POSTGRESQL__HOST: postgresql
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: {{DB_PASSWORD}}
      AUTHENTIK_POSTGRESQL__NAME: authentik
      AUTHENTIK_SECRET_KEY: {{SECRET_KEY}}
    depends_on:
      - postgresql
      - redis

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY",  label: "Secret Key",     default: None, required: true, secret: true },
        ],
    },

    // ── Automation ────────────────────────────────────────────────────────────

    Template {
        id: "n8n",
        name: "n8n",
        description: "Automação low-code de tarefas e integrações (Postgres)",
        category: TemplateCategory::Automation,
        default_port: 5678,
        compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: n8n
      POSTGRES_USER: n8n
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  n8n:
    image: n8nio/n8n:latest
    restart: unless-stopped
    ports:
      - "5678"
    environment:
      DB_TYPE: postgresdb
      DB_POSTGRESDB_HOST: db
      DB_POSTGRESDB_DATABASE: n8n
      DB_POSTGRESDB_USER: n8n
      DB_POSTGRESDB_PASSWORD: {{DB_PASSWORD}}
      N8N_BASIC_AUTH_ACTIVE: "true"
      N8N_BASIC_AUTH_USER: {{ADMIN_USER}}
      N8N_BASIC_AUTH_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - n8n_data:/home/node/.n8n
    depends_on:
      - db

volumes:
  db_data:
  n8n_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD",     label: "Senha do banco", default: None,          required: true, secret: true  },
            TemplateVar { key: "ADMIN_USER",      label: "Usuário admin",  default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "ADMIN_PASSWORD",  label: "Senha admin",    default: None,          required: true, secret: true  },
        ],
    },

    Template {
        id: "listmonk",
        name: "Listmonk",
        description: "Gerenciador de newsletters e e-mail marketing (Postgres)",
        category: TemplateCategory::Automation,
        default_port: 9000,
        compose: r#"
services:
  db:
    image: postgres:13
    restart: unless-stopped
    environment:
      POSTGRES_DB: listmonk
      POSTGRES_USER: listmonk
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  listmonk:
    image: listmonk/listmonk:latest
    restart: unless-stopped
    ports:
      - "9000"
    environment:
      LISTMONK_db__host: db
      LISTMONK_db__port: "5432"
      LISTMONK_db__user: listmonk
      LISTMONK_db__password: {{DB_PASSWORD}}
      LISTMONK_db__database: listmonk
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    // ── Media ─────────────────────────────────────────────────────────────────

    Template {
        id: "immich",
        name: "Immich",
        description: "Backup de fotos e vídeos do celular com IA",
        category: TemplateCategory::Media,
        default_port: 2283,
        compose: r#"
services:
  immich-server:
    image: ghcr.io/immich-app/immich-server:release
    restart: unless-stopped
    ports:
      - "2283"
    environment:
      DB_HOSTNAME: database
      DB_USERNAME: immich
      DB_PASSWORD: {{DB_PASSWORD}}
      DB_DATABASE_NAME: immich
      REDIS_HOSTNAME: redis
    volumes:
      - upload_data:/usr/src/app/upload
    depends_on:
      - database
      - redis

  immich-machine-learning:
    image: ghcr.io/immich-app/immich-machine-learning:release
    restart: unless-stopped
    volumes:
      - model_cache:/cache

  database:
    image: tensorchord/pgvecto-rs:pg14-v0.2.0
    restart: unless-stopped
    environment:
      POSTGRES_DB: immich
      POSTGRES_USER: immich
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  redis:
    image: redis:7
    restart: unless-stopped

volumes:
  upload_data:
  model_cache:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "jellyfin",
        name: "Jellyfin",
        description: "Servidor de mídia e streaming gratuito e open-source",
        category: TemplateCategory::Media,
        default_port: 8096,
        compose: r#"
services:
  jellyfin:
    image: jellyfin/jellyfin:latest
    restart: unless-stopped
    ports:
      - "8096"
    volumes:
      - config:/config
      - cache:/cache

volumes:
  config:
  cache:
"#,
        variables: &[],
    },

    Template {
        id: "linkding",
        name: "Linkding",
        description: "Gerenciador de favoritos veloz e minimalista",
        category: TemplateCategory::DevTools,
        default_port: 9090,
        compose: r#"
services:
  linkding:
    image: sissbruecker/linkding:latest
    restart: unless-stopped
    ports:
      - "9090"
    environment:
      LD_SUPERUSER_NAME: {{ADMIN_USER}}
      LD_SUPERUSER_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - linkding_data:/etc/linkding/data

volumes:
  linkding_data:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_USER",     label: "Usuário admin", default: Some("admin"), required: true,  secret: false },
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin",   default: None,          required: true,  secret: true  },
        ],
    },

    Template {
        id: "pocketbase",
        name: "PocketBase",
        description: "Backend completo em arquivo único com SQLite e realtime",
        category: TemplateCategory::DevTools,
        default_port: 8090,
        compose: r#"
services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:latest
    restart: unless-stopped
    ports:
      - "8090"
    volumes:
      - pb_data:/pb_data

volumes:
  pb_data:
"#,
        variables: &[],
    },

    Template {
        id: "activepieces",
        name: "Activepieces",
        description: "Automação no-code (Alternativa ao Zapier)",
        category: TemplateCategory::Automation,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: activepieces
      POSTGRES_USER: activepieces
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  activepieces:
    image: activepieces/activepieces:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://activepieces:{{DB_PASSWORD}}@db:5432/activepieces
      AP_ENCRYPTION_KEY: {{AP_ENCRYPTION_KEY}}
      AP_JWT_SECRET: {{AP_JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "AP_ENCRYPTION_KEY", label: "Chave de criptografia", default: None, required: true, secret: true },
            TemplateVar { key: "AP_JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "actual-budget",
        name: "Actual Budget",
        description: "Gerenciador de finanças pessoais rápido e privado",
        category: TemplateCategory::Finance,
        default_port: 5006,
        compose: r#"services:
  actual-budget:
    image: actualbudget/actual-server:latest
    restart: unless-stopped
    ports:
      - "5006"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "adguard-home",
        name: "AdGuard Home",
        description: "Bloqueador de anúncios e rastreadores em nível de DNS",
        category: TemplateCategory::Networking,
        default_port: 3000,
        compose: r#"services:
  adguard-home:
    image: adguard/adguardhome:latest
    restart: unless-stopped
    ports:
      - "3000"
    volumes:
      - workdir:/opt/adguardhome/work
      - confdir:/opt/adguardhome/conf

volumes:
  workdir:
  confdir:
"#,
        variables: &[

        ],
    },

    Template {
        id: "adminer",
        name: "Adminer",
        description: "Gerenciador de banco de dados leve (MySQL, Postgres, SQLite)",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  adminer:
    image: adminer:latest
    restart: unless-stopped
    ports:
      - "8080"
"#,
        variables: &[

        ],
    },

    Template {
        id: "anythingllm",
        name: "AnythingLLM",
        description: "Chatbot privado para conversar com seus documentos locais",
        category: TemplateCategory::Ai,
        default_port: 3001,
        compose: r#"services:
  anythingllm:
    image: mintplexlabs/anythingllm:latest
    restart: unless-stopped
    ports:
      - "3001"
    volumes:
      - storage:/app/server/storage

volumes:
  storage:
"#,
        variables: &[

        ],
    },

    Template {
        id: "appwrite",
        name: "Appwrite",
        description: "Backend-as-a-Service (BaaS) completo em Docker",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  appwrite:
    image: appwrite/appwrite:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      _APP_ENV: {{_APP_ENV}}
      _APP_OPENSSL_KEY_V1: {{_APP_OPENSSL_KEY_V1}}
"#,
        variables: &[
            TemplateVar { key: "_APP_ENV", label: "Ambiente", default: Some("production"), required: false, secret: false },
            TemplateVar { key: "_APP_OPENSSL_KEY_V1", label: "OpenSSL Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "arangodb",
        name: "ArangoDB",
        description: "Banco de dados multi-modelo (grafos, documentos, KV)",
        category: TemplateCategory::Database,
        default_port: 8529,
        compose: r#"services:
  arangodb:
    image: arangodb:latest
    restart: unless-stopped
    ports:
      - "8529"
    environment:
      ARANGO_ROOT_PASSWORD: {{ARANGO_ROOT_PASSWORD}}
    volumes:
      - data:/var/lib/arangodb3

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "ARANGO_ROOT_PASSWORD", label: "Senha root", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "audiobookshelf",
        name: "Audiobookshelf",
        description: "Servidor de mídia para audiolivros e podcasts",
        category: TemplateCategory::Media,
        default_port: 13378,
        compose: r#"services:
  audiobookshelf:
    image: ghcr.io/advplyr/audiobookshelf:latest
    restart: unless-stopped
    ports:
      - "13378"
    volumes:
      - config:/config
      - metadata:/metadata
      - audiobooks:/audiobooks

volumes:
  config:
  metadata:
  audiobooks:
"#,
        variables: &[

        ],
    },

    Template {
        id: "authelia",
        name: "Authelia",
        description: "Provedor de SSO com autenticação multifator (2FA)",
        category: TemplateCategory::Security,
        default_port: 9091,
        compose: r#"services:
  authelia:
    image: authelia/authelia:latest
    restart: unless-stopped
    ports:
      - "9091"
    volumes:
      - config:/config

volumes:
  config:
"#,
        variables: &[

        ],
    },

    Template {
        id: "azuracast",
        name: "AzuraCast",
        description: "Painel completo para gerenciamento de Web Rádios",
        category: TemplateCategory::Media,
        default_port: 80,
        compose: r#"services:
  azuracast:
    image: ghcr.io/azuracast/azuracast:latest
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - station_data:/var/azuracast/stations

volumes:
  station_data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "backrest",
        name: "Backrest",
        description: "Interface web para backups automatizados via restic",
        category: TemplateCategory::Backup,
        default_port: 9898,
        compose: r#"services:
  backrest:
    image: garethgeorge/backrest:latest
    restart: unless-stopped
    ports:
      - "9898"
    volumes:
      - data:/data
      - config:/etc/backrest
      - cache:/var/cache/backrest

volumes:
  data:
  config:
  cache:
"#,
        variables: &[

        ],
    },

    Template {
        id: "baikal",
        name: "Baikal",
        description: "Servidor CalDAV e CardDAV leve",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  baikal:
    image: ckulka/baikal:nginx
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - config:/var/www/html/config
      - data:/var/www/html/Specific

volumes:
  config:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "baserow",
        name: "Baserow",
        description: "Banco de dados relacional com interface de planilha (Alternativa ao Airtable)",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: baserow
      POSTGRES_USER: baserow
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  baserow:
    image: baserow/baserow:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DATABASE_URL: postgresql://baserow:{{DB_PASSWORD}}@db:5432/baserow
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - uploads:/baserow/media/user_files
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "beszel",
        name: "Beszel",
        description: "Monitor leve de servidores com estatísticas de containers",
        category: TemplateCategory::Monitoring,
        default_port: 8090,
        compose: r#"services:
  beszel:
    image: henrygd/beszel:latest
    restart: unless-stopped
    ports:
      - "8090"
    volumes:
      - data:/beszel_data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "bookstack",
        name: "BookStack",
        description: "Plataforma wiki para documentações corporativas",
        category: TemplateCategory::Cms,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: bookstack
      MYSQL_USER: bookstack
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  bookstack:
    image: lscr.io/linuxserver/bookstack:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: bookstack
      DB_USER: bookstack
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
      APP_URL: {{APP_URL}}
    volumes:
      - uploads:/config
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_KEY", label: "App Key", default: None, required: true, secret: true },
            TemplateVar { key: "APP_URL", label: "URL da aplicação", default: Some("http://localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "botpress",
        name: "Botpress",
        description: "Plataforma para criação de agentes de IA conversacionais",
        category: TemplateCategory::Ai,
        default_port: 3000,
        compose: r#"services:
  botpress:
    image: botpress/server:latest
    restart: unless-stopped
    ports:
      - "3000"
    volumes:
      - data:/botpress/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "browserless",
        name: "Browserless",
        description: "Execução remota e headless do Chrome/Puppeteer em containers",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  browserless:
    image: browserless/chrome:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      TOKEN: {{TOKEN}}
"#,
        variables: &[
            TemplateVar { key: "TOKEN", label: "API Token", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "budibase",
        name: "Budibase",
        description: "Plataforma low-code para criação de ferramentas internas",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: budibase
      POSTGRES_USER: budibase
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  budibase:
    image: budibase/budibase:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DATABASE_URL: postgresql://budibase:{{DB_PASSWORD}}@db:5432/budibase
      JWT_SECRET: {{JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "bytebase",
        name: "Bytebase",
        description: "Ferramenta para controle do ciclo de vida de bancos de dados",
        category: TemplateCategory::DevTools,
        default_port: 5678,
        compose: r#"services:
  bytebase:
    image: bytebase/bytebase:latest
    restart: unless-stopped
    ports:
      - "5678"
    volumes:
      - data:/var/opt/bytebase

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "bytestash",
        name: "ByteStash",
        description: "Repositório privado e organizador de trechos de código",
        category: TemplateCategory::DevTools,
        default_port: 5000,
        compose: r#"services:
  bytestash:
    image: ghcr.io/codeharbour/bytestash:latest
    restart: unless-stopped
    ports:
      - "5000"
    environment:
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - data:/app/db

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "calibre",
        name: "Calibre",
        description: "Gerenciador e organizador de bibliotecas de e-books",
        category: TemplateCategory::Media,
        default_port: 8080,
        compose: r#"services:
  calibre:
    image: lscr.io/linuxserver/calibre:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      PASSWORD: {{PASSWORD}}
    volumes:
      - config:/config

volumes:
  config:
"#,
        variables: &[
            TemplateVar { key: "PASSWORD", label: "Senha de acesso", default: None, required: false, secret: true },
        ],
    },

    Template {
        id: "calibre-web",
        name: "Calibre-Web",
        description: "Interface para e-books do Calibre via navegador",
        category: TemplateCategory::Media,
        default_port: 8083,
        compose: r#"services:
  calibre-web:
    image: lscr.io/linuxserver/calibre-web:latest
    restart: unless-stopped
    ports:
      - "8083"
    volumes:
      - config:/config
      - books:/books

volumes:
  config:
  books:
"#,
        variables: &[

        ],
    },

    Template {
        id: "change-detection",
        name: "changedetection.io",
        description: "Monitor inteligente para alterações em páginas web",
        category: TemplateCategory::Monitoring,
        default_port: 5000,
        compose: r#"services:
  change-detection:
    image: ghcr.io/dgtlmoon/changedetection.io:latest
    restart: unless-stopped
    ports:
      - "5000"
    volumes:
      - datastore:/datastore

volumes:
  datastore:
"#,
        variables: &[

        ],
    },

    Template {
        id: "chatwoot",
        name: "Chatwoot",
        description: "Plataforma de atendimento omnichannel (Live Chat, WhatsApp)",
        category: TemplateCategory::Communication,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: chatwoot
      POSTGRES_USER: chatwoot
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  chatwoot:
    image: chatwoot/chatwoot:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://chatwoot:{{DB_PASSWORD}}@db:5432/chatwoot
      SECRET_KEY_BASE: {{SECRET_KEY_BASE}}
    volumes:
      - storage:/app/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY_BASE", label: "Secret Key Base", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "clickhouse",
        name: "ClickHouse",
        description: "Banco de dados analítico (OLAP) orientado a colunas extremamente veloz",
        category: TemplateCategory::Database,
        default_port: 8123,
        compose: r#"services:
  clickhouse:
    image: clickhouse/clickhouse-server:latest
    restart: unless-stopped
    ports:
      - "8123"
    volumes:
      - data:/var/lib/clickhouse

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "cloudflared",
        name: "Cloudflared",
        description: "Daemon para conectar serviços locais via Cloudflare Tunnel",
        category: TemplateCategory::Networking,
        default_port: 0,
        compose: r#"services:
  cloudflared:
    image: cloudflare/cloudflared:latest
    restart: unless-stopped
    environment:
      TUNNEL_TOKEN: {{TUNNEL_TOKEN}}
"#,
        variables: &[
            TemplateVar { key: "TUNNEL_TOKEN", label: "Token do Tunnel", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "conduit",
        name: "Conduit",
        description: "Servidor de chat Matrix ultrarrápido escrito em Rust",
        category: TemplateCategory::Communication,
        default_port: 6167,
        compose: r#"services:
  conduit:
    image: registry.gitlab.com/famedly/conduit:latest
    restart: unless-stopped
    ports:
      - "6167"
    environment:
      CONDUIT_SERVER_NAME: {{CONDUIT_SERVER_NAME}}
      CONDUIT_REGISTRATION_TOKEN: {{CONDUIT_REGISTRATION_TOKEN}}
    volumes:
      - data:/var/lib/matrix-conduit

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "CONDUIT_SERVER_NAME", label: "Nome do servidor", default: None, required: true, secret: false },
            TemplateVar { key: "CONDUIT_REGISTRATION_TOKEN", label: "Token de registro", default: None, required: false, secret: true },
        ],
    },

    Template {
        id: "couchdb",
        name: "CouchDB",
        description: "Banco de dados NoSQL baseado em documentos com boa sincronização",
        category: TemplateCategory::Database,
        default_port: 5984,
        compose: r#"services:
  couchdb:
    image: couchdb:latest
    restart: unless-stopped
    ports:
      - "5984"
    environment:
      COUCHDB_USER: {{COUCHDB_USER}}
      COUCHDB_PASSWORD: {{COUCHDB_PASSWORD}}
    volumes:
      - data:/opt/couchdb/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "COUCHDB_USER", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "COUCHDB_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "crowdsec",
        name: "CrowdSec",
        description: "Sistema de segurança colaborativo contra IPs maliciosos",
        category: TemplateCategory::Security,
        default_port: 8080,
        compose: r#"services:
  crowdsec:
    image: crowdsecurity/crowdsec:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - config:/etc/crowdsec
      - data:/var/lib/crowdsec/data

volumes:
  config:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "cyberchef",
        name: "CyberChef",
        description: "Canivete suíço web para criptografia e análise de dados",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  cyberchef:
    image: mpepping/cyberchef:latest
    restart: unless-stopped
    ports:
      - "8080"
"#,
        variables: &[

        ],
    },

    Template {
        id: "dashy",
        name: "Dashy",
        description: "Dashboard pessoal customizável com monitoramento de status",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  dashy:
    image: lissy93/dashy:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - conf.yml:/app/public/conf.yml

volumes:
  conf.yml:
"#,
        variables: &[

        ],
    },

    Template {
        id: "directus",
        name: "Directus",
        description: "CMS Headless e wrapper de APIs para SQL",
        category: TemplateCategory::Cms,
        default_port: 8055,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: directus
      POSTGRES_USER: directus
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  directus:
    image: directus/directus:latest
    restart: unless-stopped
    ports:
      - "8055"
    environment:
      DATABASE_URL: postgresql://directus:{{DB_PASSWORD}}@db:5432/directus
      SECRET: {{SECRET}}
      ADMIN_EMAIL: {{ADMIN_EMAIL}}
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - uploads:/directus/uploads
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET", label: "Chave secreta", default: None, required: true, secret: true },
            TemplateVar { key: "ADMIN_EMAIL", label: "Email admin", default: Some("admin@example.com"), required: true, secret: false },
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "docmost",
        name: "Docmost",
        description: "Wiki colaborativa open-source para equipes",
        category: TemplateCategory::Cms,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: docmost
      POSTGRES_USER: docmost
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  docmost:
    image: docmost/docmost:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://docmost:{{DB_PASSWORD}}@db:5432/docmost
      APP_SECRET: {{APP_SECRET}}
    volumes:
      - storage:/app/data/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_SECRET", label: "App Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "docker-registry",
        name: "Docker Registry",
        description: "Servidor de distribuição oficial para imagens Docker",
        category: TemplateCategory::DevTools,
        default_port: 5000,
        compose: r#"services:
  docker-registry:
    image: registry:2
    restart: unless-stopped
    ports:
      - "5000"
    volumes:
      - data:/var/lib/registry

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "dolibarr",
        name: "Dolibarr",
        description: "Pacote ERP e CRM para gestão empresarial",
        category: TemplateCategory::Finance,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: dolibarr
      MYSQL_USER: dolibarr
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  dolibarr:
    image: dolibarr/dolibarr:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: dolibarr
      DB_USER: dolibarr
      DB_PASSWORD: {{DB_PASSWORD}}
      DOLI_ADMIN_LOGIN: {{DOLI_ADMIN_LOGIN}}
      DOLI_ADMIN_PASSWORD: {{DOLI_ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "DOLI_ADMIN_LOGIN", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "DOLI_ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "dragonfly",
        name: "Dragonfly",
        description: "Substituto drop-in de alta performance para o Redis",
        category: TemplateCategory::Database,
        default_port: 6379,
        compose: r#"services:
  dragonfly:
    image: docker.dragonflydb.io/dragonflydb/dragonfly:latest
    restart: unless-stopped
    ports:
      - "6379"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "drawio",
        name: "draw.io",
        description: "Ferramenta para desenho de diagramas e quadros brancos",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  drawio:
    image: jgraph/drawio:latest
    restart: unless-stopped
    ports:
      - "8080"
"#,
        variables: &[

        ],
    },

    Template {
        id: "elasticsearch",
        name: "Elasticsearch",
        description: "Motor distribuído de busca textual e análise analítica",
        category: TemplateCategory::Database,
        default_port: 9200,
        compose: r#"services:
  elasticsearch:
    image: elasticsearch:8.11.1
    restart: unless-stopped
    ports:
      - "9200"
    environment:
      ELASTIC_PASSWORD: {{ELASTIC_PASSWORD}}
      discovery.type: {{discovery.type}}
    volumes:
      - data:/usr/share/elasticsearch/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "ELASTIC_PASSWORD", label: "Senha Elasticsearch", default: None, required: true, secret: true },
            TemplateVar { key: "discovery.type", label: "Modo single-node", default: Some("single-node"), required: false, secret: false },
        ],
    },

    Template {
        id: "emby",
        name: "Emby",
        description: "Servidor de mídia privado para streaming de filmes e músicas",
        category: TemplateCategory::Media,
        default_port: 8096,
        compose: r#"services:
  emby:
    image: emby/embyserver:latest
    restart: unless-stopped
    ports:
      - "8096"
    volumes:
      - config:/config
      - cache:/cache

volumes:
  config:
  cache:
"#,
        variables: &[

        ],
    },

    Template {
        id: "emqx",
        name: "EMQX",
        description: "Broker MQTT massivamente escalável para projetos IoT",
        category: TemplateCategory::Networking,
        default_port: 1883,
        compose: r#"services:
  emqx:
    image: emqx:latest
    restart: unless-stopped
    ports:
      - "1883"
    volumes:
      - data:/opt/emqx/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "etherpad",
        name: "Etherpad",
        description: "Editor de texto colaborativo multiusuário em tempo real",
        category: TemplateCategory::DevTools,
        default_port: 9001,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: etherpad
      POSTGRES_USER: etherpad
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  etherpad:
    image: etherpad/etherpad:latest
    restart: unless-stopped
    ports:
      - "9001"
    environment:
      DATABASE_URL: postgresql://etherpad:{{DB_PASSWORD}}@db:5432/etherpad
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "evolution-api",
        name: "Evolution API",
        description: "API de WhatsApp focada em automação para empresas",
        category: TemplateCategory::Automation,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: evolution_api
      POSTGRES_USER: evolution_api
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  evolution-api:
    image: atendai/evolution-api:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://evolution_api:{{DB_PASSWORD}}@db:5432/evolution_api
      AUTHENTICATION_API_KEY: {{AUTHENTICATION_API_KEY}}
    volumes:
      - instances:/evolution/instances
    depends_on:
      - db

volumes:
  db_data:
  instances:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "AUTHENTICATION_API_KEY", label: "API Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "excalidraw",
        name: "Excalidraw",
        description: "Quadro branco virtual para esboços e diagramas",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  excalidraw:
    image: excalidraw/excalidraw:latest
    restart: unless-stopped
    ports:
      - "80"
"#,
        variables: &[

        ],
    },

    Template {
        id: "ezbookkeeping",
        name: "EZBookkeeping",
        description: "Gerenciador contábil para finanças pessoais",
        category: TemplateCategory::Finance,
        default_port: 8080,
        compose: r#"services:
  ezbookkeeping:
    image: mayswind/ezbookkeeping:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      EZB_SECRET_KEY: {{EZB_SECRET_KEY}}
    volumes:
      - data:/ezbookkeeping/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "EZB_SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "flaresolver",
        name: "FlareSolverr",
        description: "Proxy para contornar proteções do Cloudflare",
        category: TemplateCategory::Networking,
        default_port: 8191,
        compose: r#"services:
  flaresolver:
    image: ghcr.io/flaresolverr/flaresolverr:latest
    restart: unless-stopped
    ports:
      - "8191"
"#,
        variables: &[

        ],
    },

    Template {
        id: "flowise",
        name: "Flowise",
        description: "Interface no-code para construir cadeias de LLM",
        category: TemplateCategory::Ai,
        default_port: 3000,
        compose: r#"services:
  flowise:
    image: flowiseai/flowise:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      FLOWISE_USERNAME: {{FLOWISE_USERNAME}}
      FLOWISE_PASSWORD: {{FLOWISE_PASSWORD}}
    volumes:
      - data:/root/.flowise

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "FLOWISE_USERNAME", label: "Usuário", default: Some("admin"), required: false, secret: false },
            TemplateVar { key: "FLOWISE_PASSWORD", label: "Senha", default: None, required: false, secret: true },
        ],
    },

    Template {
        id: "focalboard",
        name: "Focalboard",
        description: "Gerenciador de tarefas Kanban (Alternativa ao Trello/Asana)",
        category: TemplateCategory::ProjectManagement,
        default_port: 8000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: focalboard
      POSTGRES_USER: focalboard
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  focalboard:
    image: mattermost/focalboard:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      DATABASE_URL: postgresql://focalboard:{{DB_PASSWORD}}@db:5432/focalboard
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "forgejo",
        name: "Forgejo",
        description: "Plataforma leve para hospedagem de código Git",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:latest
    restart: unless-stopped
    ports:
      - "3000"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "freshrss",
        name: "FreshRSS",
        description: "Agregador e leitor de feeds RSS rápido e customizável",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  freshrss:
    image: freshrss/freshrss:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - data:/var/www/FreshRSS/data
      - extensions:/var/www/FreshRSS/extensions

volumes:
  data:
  extensions:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "garage-s3",
        name: "Garage S3",
        description: "Armazenamento de objetos distribuído compatível com S3",
        category: TemplateCategory::Storage,
        default_port: 3900,
        compose: r#"services:
  garage-s3:
    image: dxflrs/garage:latest
    restart: unless-stopped
    ports:
      - "3900"
    volumes:
      - data:/var/lib/garage/data
      - meta:/var/lib/garage/meta

volumes:
  data:
  meta:
"#,
        variables: &[

        ],
    },

    Template {
        id: "gitea-mysql",
        name: "Gitea (MySQL)",
        description: "Servidor Git Gitea com banco de dados MySQL",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: gitea_mysql
      MYSQL_USER: gitea_mysql
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  gitea-mysql:
    image: gitea/gitea:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DB_HOST: db
      DB_NAME: gitea_mysql
      DB_USER: gitea_mysql
      DB_PASSWORD: {{DB_PASSWORD}}
      GITEA__server__DOMAIN: {{GITEA__server__DOMAIN}}
    volumes:
      - data:/data
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "GITEA__server__DOMAIN", label: "Domínio", default: Some("localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "gitlab-ce",
        name: "GitLab CE",
        description: "Plataforma DevOps completa para código e CI/CD",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  gitlab-ce:
    image: gitlab/gitlab-ce:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      GITLAB_ROOT_PASSWORD: {{GITLAB_ROOT_PASSWORD}}
      GITLAB_OMNIBUS_CONFIG: {{GITLAB_OMNIBUS_CONFIG}}
    volumes:
      - config:/etc/gitlab
      - logs:/var/log/gitlab
      - data:/var/opt/gitlab

volumes:
  config:
  logs:
  data:
"#,
        variables: &[
            TemplateVar { key: "GITLAB_ROOT_PASSWORD", label: "Senha root", default: None, required: true, secret: true },
            TemplateVar { key: "GITLAB_OMNIBUS_CONFIG", label: "Hostname config", default: Some("external_url 'http://localhost'"), required: true, secret: false },
        ],
    },

    Template {
        id: "glitchtip",
        name: "GlitchTip",
        description: "Coletor centralizado de erros (Alternativa ao Sentry)",
        category: TemplateCategory::Monitoring,
        default_port: 8000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: glitchtip
      POSTGRES_USER: glitchtip
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  glitchtip:
    image: glitchtip/glitchtip:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      DATABASE_URL: postgresql://glitchtip:{{DB_PASSWORD}}@db:5432/glitchtip
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - uploads:/code/uploads
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "gotenberg",
        name: "Gotenberg",
        description: "API escalável para conversões de arquivos para PDF",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  gotenberg:
    image: gotenberg/gotenberg:latest
    restart: unless-stopped
    ports:
      - "3000"
"#,
        variables: &[

        ],
    },

    Template {
        id: "grist",
        name: "Grist",
        description: "Planilha inteligente integrada com banco de dados relacional",
        category: TemplateCategory::DevTools,
        default_port: 8484,
        compose: r#"services:
  grist:
    image: gristlabs/grist:latest
    restart: unless-stopped
    ports:
      - "8484"
    environment:
      GRIST_SESSION_SECRET: {{GRIST_SESSION_SECRET}}
    volumes:
      - data:/persist

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "GRIST_SESSION_SECRET", label: "Session Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "grimoire",
        name: "Grimoire",
        description: "Organizador e salvador de favoritos (bookmarks) ultra veloz",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: grimoire
      POSTGRES_USER: grimoire
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  grimoire:
    image: ghcr.io/goniszewski/grimoire:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://grimoire:{{DB_PASSWORD}}@db:5432/grimoire
      SECRET: {{SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET", label: "Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "homarr",
        name: "Homarr",
        description: "Dashboard moderno de aplicativos residenciais integrado ao Docker",
        category: TemplateCategory::DevTools,
        default_port: 7575,
        compose: r#"services:
  homarr:
    image: ghcr.io/ajnart/homarr:latest
    restart: unless-stopped
    ports:
      - "7575"
    volumes:
      - configs:/app/data/configs
      - icons:/app/public/icons
      - data:/data

volumes:
  configs:
  icons:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "homeassistant",
        name: "Home Assistant",
        description: "Ecossistema open-source definitivo para automação residencial",
        category: TemplateCategory::Networking,
        default_port: 8123,
        compose: r#"services:
  homeassistant:
    image: homeassistant/home-assistant:latest
    restart: unless-stopped
    ports:
      - "8123"
    volumes:
      - config:/config

volumes:
  config:
"#,
        variables: &[

        ],
    },

    Template {
        id: "hoarder",
        name: "Hoarder",
        description: "Bookmarks inteligentes com auto-tagging baseado em IA",
        category: TemplateCategory::Ai,
        default_port: 3000,
        compose: r#"services:
  hoarder:
    image: ghcr.io/hoarder-app/hoarder:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      MEILI_MASTER_KEY: {{MEILI_MASTER_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "NEXTAUTH_SECRET", label: "NextAuth Secret", default: None, required: true, secret: true },
            TemplateVar { key: "MEILI_MASTER_KEY", label: "Meilisearch Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "hoppscotch",
        name: "Hoppscotch",
        description: "Suíte completa de testes de API (Alternativa ao Postman)",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: hoppscotch
      POSTGRES_USER: hoppscotch
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  hoppscotch:
    image: hoppscotch/hoppscotch:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://hoppscotch:{{DB_PASSWORD}}@db:5432/hoppscotch
      JWT_SECRET: {{JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "huly",
        name: "Huly",
        description: "Gerenciador de projetos (Alternativa ao Jira/Linear/Slack)",
        category: TemplateCategory::ProjectManagement,
        default_port: 8083,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: huly
      POSTGRES_USER: huly
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  huly:
    image: hardcoreeng/huly:latest
    restart: unless-stopped
    ports:
      - "8083"
    environment:
      DATABASE_URL: postgresql://huly:{{DB_PASSWORD}}@db:5432/huly
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "influxdb",
        name: "InfluxDB",
        description: "Banco de dados otimizado para séries temporais",
        category: TemplateCategory::Database,
        default_port: 8086,
        compose: r#"services:
  influxdb:
    image: influxdb:latest
    restart: unless-stopped
    ports:
      - "8086"
    environment:
      DOCKER_INFLUXDB_INIT_USERNAME: {{DOCKER_INFLUXDB_INIT_USERNAME}}
      DOCKER_INFLUXDB_INIT_PASSWORD: {{DOCKER_INFLUXDB_INIT_PASSWORD}}
      DOCKER_INFLUXDB_INIT_ORG: {{DOCKER_INFLUXDB_INIT_ORG}}
      DOCKER_INFLUXDB_INIT_BUCKET: {{DOCKER_INFLUXDB_INIT_BUCKET}}
    volumes:
      - data:/var/lib/influxdb2

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "DOCKER_INFLUXDB_INIT_USERNAME", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "DOCKER_INFLUXDB_INIT_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
            TemplateVar { key: "DOCKER_INFLUXDB_INIT_ORG", label: "Organização", default: Some("myorg"), required: true, secret: false },
            TemplateVar { key: "DOCKER_INFLUXDB_INIT_BUCKET", label: "Bucket padrão", default: Some("mybucket"), required: true, secret: false },
        ],
    },

    Template {
        id: "invoiceshelf",
        name: "InvoiceShelf",
        description: "Emissor de faturas para profissionais autônomos",
        category: TemplateCategory::Finance,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: invoiceshelf
      MYSQL_USER: invoiceshelf
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  invoiceshelf:
    image: invoiceshelf/invoiceshelf:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: invoiceshelf
      DB_USER: invoiceshelf
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
    volumes:
      - storage:/var/www/html/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_KEY", label: "App Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "it-tools",
        name: "IT Tools",
        description: "Coleção de utilitários online essenciais para desenvolvedores",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  it-tools:
    image: corentinth/it-tools:latest
    restart: unless-stopped
    ports:
      - "80"
"#,
        variables: &[

        ],
    },

    Template {
        id: "jenkins",
        name: "Jenkins",
        description: "Servidor de automação open-source para pipelines CI/CD",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  jenkins:
    image: jenkins/jenkins:lts
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - data:/var/jenkins_home

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "kaneo",
        name: "Kaneo",
        description: "Plataforma limpa e simplificada de gerenciamento de projetos",
        category: TemplateCategory::ProjectManagement,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kaneo
      POSTGRES_USER: kaneo
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kaneo:
    image: kaneo/kaneo:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://kaneo:{{DB_PASSWORD}}@db:5432/kaneo
      JWT_SECRET: {{JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "kener",
        name: "Kener",
        description: "Página de status open-source moderna para monitoramento",
        category: TemplateCategory::Monitoring,
        default_port: 3000,
        compose: r#"services:
  kener:
    image: rajnandan1/kener:latest
    restart: unless-stopped
    ports:
      - "3000"
    volumes:
      - data:/app/db

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "kestra",
        name: "Kestra",
        description: "Orquestrador declarativo de fluxos de dados e negócios",
        category: TemplateCategory::Automation,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kestra
      POSTGRES_USER: kestra
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kestra:
    image: kestra/kestra:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://kestra:{{DB_PASSWORD}}@db:5432/kestra
    volumes:
      - storage:/app/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "keycloak",
        name: "Keycloak",
        description: "Provedor robusto de gerenciamento de identidade e autenticação",
        category: TemplateCategory::Security,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: keycloak
      POSTGRES_USER: keycloak
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  keycloak:
    image: quay.io/keycloak/keycloak:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://keycloak:{{DB_PASSWORD}}@db:5432/keycloak
      KEYCLOAK_ADMIN: {{KEYCLOAK_ADMIN}}
      KEYCLOAK_ADMIN_PASSWORD: {{KEYCLOAK_ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "KEYCLOAK_ADMIN", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "KEYCLOAK_ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "kimai",
        name: "Kimai",
        description: "Sistema multiusuário para controle de horas trabalhadas",
        category: TemplateCategory::Finance,
        default_port: 8001,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: kimai
      MYSQL_USER: kimai
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  kimai:
    image: kimai/kimai2:apache
    restart: unless-stopped
    ports:
      - "8001"
    environment:
      DB_HOST: db
      DB_NAME: kimai
      DB_USER: kimai
      DB_PASSWORD: {{DB_PASSWORD}}
      ADMINMAIL: {{ADMINMAIL}}
      ADMINPASS: {{ADMINPASS}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "ADMINMAIL", label: "Email admin", default: Some("admin@example.com"), required: true, secret: false },
            TemplateVar { key: "ADMINPASS", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "kitchenowl",
        name: "KitchenOwl",
        description: "Organizador inteligente de receitas e listas de supermercado",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  kitchenowl:
    image: tombursch/kitchenowl:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      JWT_SECRET_KEY: {{JWT_SECRET_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "JWT_SECRET_KEY", label: "JWT Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "kutt",
        name: "Kutt",
        description: "Encurtador de URLs moderno com analytics e domínios customizados",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kutt
      POSTGRES_USER: kutt
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kutt:
    image: kutt/kutt:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://kutt:{{DB_PASSWORD}}@db:5432/kutt
      JWT_SECRET: {{JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "langflow",
        name: "Langflow",
        description: "Interface low-code para pipelines de RAG e agentes de IA",
        category: TemplateCategory::Ai,
        default_port: 7860,
        compose: r#"services:
  langflow:
    image: langflowai/langflow:latest
    restart: unless-stopped
    ports:
      - "7860"
    environment:
      LANGFLOW_SECRET_KEY: {{LANGFLOW_SECRET_KEY}}
    volumes:
      - data:/app/langflow

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "LANGFLOW_SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "librechat",
        name: "LibreChat",
        description: "Interface unificada para múltiplos provedores de IA",
        category: TemplateCategory::Ai,
        default_port: 3080,
        compose: r#"services:
  db:
    image: mongo:6
    restart: unless-stopped
    volumes:
      - db_data:/data/db
  librechat:
    image: ghcr.io/danny-avila/librechat:latest
    restart: unless-stopped
    ports:
      - "3080"
    environment:
      MONGO_URL: mongodb://db:27017/librechat
      JWT_SECRET: {{JWT_SECRET}}
      JWT_REFRESH_SECRET: {{JWT_REFRESH_SECRET}}
    volumes:
      - images:/app/client/public/images
    depends_on:
      - db

volumes:
  db_data:
  images:
"#,
        variables: &[
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_REFRESH_SECRET", label: "JWT Refresh Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "libretranslate",
        name: "LibreTranslate",
        description: "API auto-hospedada de tradução sem dependências externas",
        category: TemplateCategory::Ai,
        default_port: 5000,
        compose: r#"services:
  libretranslate:
    image: libretranslate/libretranslate:latest
    restart: unless-stopped
    ports:
      - "5000"
    volumes:
      - data:/home/libretranslate/.local/share

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "linkstack",
        name: "LinkStack",
        description: "Plataforma estilo Link na Bio altamente customizável",
        category: TemplateCategory::Cms,
        default_port: 80,
        compose: r#"services:
  linkstack:
    image: linkstackorg/linkstack:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - data:/htdocs

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "linkwarden",
        name: "Linkwarden",
        description: "Gerenciador de links focado em arquivamento de páginas web",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: linkwarden
      POSTGRES_USER: linkwarden
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  linkwarden:
    image: ghcr.io/linkwarden/linkwarden:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://linkwarden:{{DB_PASSWORD}}@db:5432/linkwarden
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      NEXTAUTH_URL: {{NEXTAUTH_URL}}
    volumes:
      - data:/data/data
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "NEXTAUTH_SECRET", label: "NextAuth Secret", default: None, required: true, secret: true },
            TemplateVar { key: "NEXTAUTH_URL", label: "URL da aplicação", default: Some("http://localhost:3000"), required: true, secret: false },
        ],
    },

    Template {
        id: "litellm",
        name: "LiteLLM",
        description: "Proxy que unifica múltiplos LLMs sob o padrão OpenAI",
        category: TemplateCategory::Ai,
        default_port: 4000,
        compose: r#"services:
  litellm:
    image: ghcr.io/berriai/litellm:main-latest
    restart: unless-stopped
    ports:
      - "4000"
    environment:
      LITELLM_MASTER_KEY: {{LITELLM_MASTER_KEY}}
"#,
        variables: &[
            TemplateVar { key: "LITELLM_MASTER_KEY", label: "Master Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "lobechat",
        name: "Lobe Chat",
        description: "Framework de chat com IA moderno com suporte a plugins de voz",
        category: TemplateCategory::Ai,
        default_port: 3210,
        compose: r#"services:
  lobechat:
    image: lobehub/lobe-chat:latest
    restart: unless-stopped
    ports:
      - "3210"
    environment:
      OPENAI_API_KEY: {{OPENAI_API_KEY}}
"#,
        variables: &[
            TemplateVar { key: "OPENAI_API_KEY", label: "OpenAI API Key", default: None, required: false, secret: true },
        ],
    },

    Template {
        id: "logto",
        name: "Logto",
        description: "Plataforma CIAM moderna para autenticação de clientes",
        category: TemplateCategory::Security,
        default_port: 3001,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: logto
      POSTGRES_USER: logto
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  logto:
    image: svhd/logto:latest
    restart: unless-stopped
    ports:
      - "3001"
    environment:
      DATABASE_URL: postgresql://logto:{{DB_PASSWORD}}@db:5432/logto
      TRUST_PROXY_HEADER: {{TRUST_PROXY_HEADER}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "TRUST_PROXY_HEADER", label: "Trust proxy header", default: Some("1"), required: false, secret: false },
        ],
    },

    Template {
        id: "mailpit",
        name: "Mailpit",
        description: "Servidor SMTP falso para testes e inspeção de e-mails",
        category: TemplateCategory::DevTools,
        default_port: 8025,
        compose: r#"services:
  mailpit:
    image: axllent/mailpit:latest
    restart: unless-stopped
    ports:
      - "8025"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "mautic",
        name: "Mautic",
        description: "Sistema completo de automação de marketing digital",
        category: TemplateCategory::Automation,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: mautic
      MYSQL_USER: mautic
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  mautic:
    image: mautic/mautic:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: mautic
      DB_USER: mautic
      DB_PASSWORD: {{DB_PASSWORD}}
      MAUTIC_ADMIN_USERNAME: {{MAUTIC_ADMIN_USERNAME}}
      MAUTIC_ADMIN_PASSWORD: {{MAUTIC_ADMIN_PASSWORD}}
    volumes:
      - data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "MAUTIC_ADMIN_USERNAME", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "MAUTIC_ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "mealie",
        name: "Mealie",
        description: "Gerenciador de receitas com importação automática de sites",
        category: TemplateCategory::DevTools,
        default_port: 9000,
        compose: r#"services:
  mealie:
    image: ghcr.io/mealie-recipes/mealie:latest
    restart: unless-stopped
    ports:
      - "9000"
    environment:
      DEFAULT_EMAIL: {{DEFAULT_EMAIL}}
      DEFAULT_PASSWORD: {{DEFAULT_PASSWORD}}
    volumes:
      - data:/app/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "DEFAULT_EMAIL", label: "Email admin", default: Some("admin@example.com"), required: true, secret: false },
            TemplateVar { key: "DEFAULT_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "memos",
        name: "Memos",
        description: "Central de notas rápidas focada em privacidade",
        category: TemplateCategory::DevTools,
        default_port: 5230,
        compose: r#"services:
  memos:
    image: neosmemo/memos:stable
    restart: unless-stopped
    ports:
      - "5230"
    volumes:
      - data:/.memos

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "metabase",
        name: "Metabase",
        description: "Plataforma fácil de BI para relatórios e dashboards",
        category: TemplateCategory::Analytics,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: metabase
      POSTGRES_USER: metabase
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  metabase:
    image: metabase/metabase:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://metabase:{{DB_PASSWORD}}@db:5432/metabase
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "metube",
        name: "MeTube",
        description: "Downloader web de vídeos do YouTube via yt-dlp",
        category: TemplateCategory::Media,
        default_port: 8081,
        compose: r#"services:
  metube:
    image: ghcr.io/alexta69/metube:latest
    restart: unless-stopped
    ports:
      - "8081"
    volumes:
      - downloads:/downloads

volumes:
  downloads:
"#,
        variables: &[

        ],
    },

    Template {
        id: "mumble",
        name: "Mumble",
        description: "Servidor de comunicação de voz com baixíssima latência para jogos",
        category: TemplateCategory::Communication,
        default_port: 64738,
        compose: r#"services:
  mumble:
    image: mumble/mumble-server:latest
    restart: unless-stopped
    ports:
      - "64738"
    environment:
      MUMBLE_SUPERUSER_PASSWORD: {{MUMBLE_SUPERUSER_PASSWORD}}
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "MUMBLE_SUPERUSER_PASSWORD", label: "Senha superuser", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "navidrome",
        name: "Navidrome",
        description: "Servidor leve de streaming de áudio compatível com Subsonic",
        category: TemplateCategory::Media,
        default_port: 4533,
        compose: r#"services:
  navidrome:
    image: deluan/navidrome:latest
    restart: unless-stopped
    ports:
      - "4533"
    volumes:
      - data:/data
      - music:/music

volumes:
  data:
  music:
"#,
        variables: &[

        ],
    },

    Template {
        id: "netdata",
        name: "Netdata",
        description: "Monitor analítico de infraestrutura em tempo real",
        category: TemplateCategory::Monitoring,
        default_port: 19999,
        compose: r#"services:
  netdata:
    image: netdata/netdata:latest
    restart: unless-stopped
    ports:
      - "19999"
    volumes:
      - netdataconfig:/etc/netdata
      - netdatalib:/var/lib/netdata
      - netdatacache:/var/cache/netdata

volumes:
  netdataconfig:
  netdatalib:
  netdatacache:
"#,
        variables: &[

        ],
    },

    Template {
        id: "nginx",
        name: "Nginx",
        description: "Servidor web de alta performance e proxy reverso",
        category: TemplateCategory::Networking,
        default_port: 80,
        compose: r#"services:
  nginx:
    image: nginx:latest
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - html:/usr/share/nginx/html

volumes:
  html:
"#,
        variables: &[

        ],
    },

    Template {
        id: "nocodb",
        name: "NocoDB",
        description: "Transforma bancos relacionais em interface estilo Airtable",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: nocodb
      POSTGRES_USER: nocodb
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  nocodb:
    image: nocodb/nocodb:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://nocodb:{{DB_PASSWORD}}@db:5432/nocodb
      NC_AUTH_JWT_SECRET: {{NC_AUTH_JWT_SECRET}}
    volumes:
      - data:/usr/app/data
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "NC_AUTH_JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "ntfy",
        name: "NTFY",
        description: "Notificações push para celulares via requisições HTTP simples",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  ntfy:
    image: binwiederhier/ntfy:latest
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - cache:/var/cache/ntfy
      - etc:/etc/ntfy

volumes:
  cache:
  etc:
"#,
        variables: &[

        ],
    },

    Template {
        id: "obsidian-livesync",
        name: "Obsidian LiveSync",
        description: "Servidor CouchDB para sincronização em tempo real das notas do Obsidian",
        category: TemplateCategory::Backup,
        default_port: 5984,
        compose: r#"services:
  obsidian-livesync:
    image: couchdb:latest
    restart: unless-stopped
    ports:
      - "5984"
    environment:
      COUCHDB_USER: {{COUCHDB_USER}}
      COUCHDB_PASSWORD: {{COUCHDB_PASSWORD}}
    volumes:
      - data:/opt/couchdb/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "COUCHDB_USER", label: "Usuário CouchDB", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "COUCHDB_PASSWORD", label: "Senha CouchDB", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "odoo",
        name: "Odoo",
        description: "Sistema modular ERP open-source para gestão de negócios globais",
        category: TemplateCategory::Finance,
        default_port: 8069,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: odoo
      POSTGRES_USER: odoo
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  odoo:
    image: odoo:latest
    restart: unless-stopped
    ports:
      - "8069"
    environment:
      DATABASE_URL: postgresql://odoo:{{DB_PASSWORD}}@db:5432/odoo
    volumes:
      - data:/var/lib/odoo
      - addons:/mnt/extra-addons
    depends_on:
      - db

volumes:
  db_data:
  data:
  addons:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "onedev",
        name: "OneDev",
        description: "Servidor Git com quadros Kanban e esteiras nativas de CI/CD",
        category: TemplateCategory::DevTools,
        default_port: 6610,
        compose: r#"services:
  onedev:
    image: 1dev/server:latest
    restart: unless-stopped
    ports:
      - "6610"
    volumes:
      - data:/opt/onedev

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "one-time-secret",
        name: "One Time Secret",
        description: "Compartilhamento seguro de segredos por links que se destroem",
        category: TemplateCategory::Security,
        default_port: 7143,
        compose: r#"services:
  redis:
    image: redis:7-alpine
    restart: unless-stopped
    volumes:
      - redis_data:/data
  one-time-secret:
    image: onetimesecret/onetimesecret:latest
    restart: unless-stopped
    ports:
      - "7143"
    environment:
      REDIS_URL: redis://redis:6379
      OTS_SECRET: {{OTS_SECRET}}
    depends_on:
      - redis

volumes:
  redis_data:
"#,
        variables: &[
            TemplateVar { key: "OTS_SECRET", label: "Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "open-webui",
        name: "Open WebUI",
        description: "Interface web para modelos LLM locais (Ollama) estilo ChatGPT",
        category: TemplateCategory::Ai,
        default_port: 3000,
        compose: r#"services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      WEBUI_SECRET_KEY: {{WEBUI_SECRET_KEY}}
    volumes:
      - data:/app/backend/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "WEBUI_SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "opengist",
        name: "OpenGist",
        description: "Alternativa ao GitHub Gist para trechos de código",
        category: TemplateCategory::DevTools,
        default_port: 6157,
        compose: r#"services:
  opengist:
    image: ghcr.io/thomiceli/opengist:latest
    restart: unless-stopped
    ports:
      - "6157"
    volumes:
      - data:/opengist

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "openresty-manager",
        name: "OpenResty Manager",
        description: "Painel para servidores Nginx/OpenResty com SSL",
        category: TemplateCategory::Networking,
        default_port: 81,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: openresty_manager
      MYSQL_USER: openresty_manager
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  openresty-manager:
    image: jc21/nginx-proxy-manager:latest
    restart: unless-stopped
    ports:
      - "81"
    environment:
      DB_HOST: db
      DB_NAME: openresty_manager
      DB_USER: openresty_manager
      DB_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - data:/data
      - letsencrypt:/etc/letsencrypt
    depends_on:
      - db

volumes:
  db_data:
  data:
  letsencrypt:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "outline",
        name: "Outline",
        description: "Base de conhecimento corporativa moderna para equipes ágeis",
        category: TemplateCategory::Cms,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: outline
      POSTGRES_USER: outline
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  outline:
    image: outlinewiki/outline:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://outline:{{DB_PASSWORD}}@db:5432/outline
      SECRET_KEY: {{SECRET_KEY}}
      UTILS_SECRET: {{UTILS_SECRET}}
    volumes:
      - data:/var/lib/outline/data
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY", label: "Secret Key (32 hex chars)", default: None, required: true, secret: true },
            TemplateVar { key: "UTILS_SECRET", label: "Utils Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "passbolt",
        name: "Passbolt",
        description: "Gerenciador de senhas open-source para equipes técnicas",
        category: TemplateCategory::Security,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: passbolt
      MYSQL_USER: passbolt
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  passbolt:
    image: passbolt/passbolt:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: passbolt
      DB_USER: passbolt
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_FULL_BASE_URL: {{APP_FULL_BASE_URL}}
    volumes:
      - gpg:/etc/passbolt/gpg
      - jwt:/etc/passbolt/jwt
    depends_on:
      - db

volumes:
  db_data:
  gpg:
  jwt:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_FULL_BASE_URL", label: "URL base", default: Some("https://localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "pgadmin",
        name: "pgAdmin",
        description: "Interface gráfica oficial para gerenciamento de PostgreSQL",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  pgadmin:
    image: dpage/pgadmin4:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      PGADMIN_DEFAULT_EMAIL: {{PGADMIN_DEFAULT_EMAIL}}
      PGADMIN_DEFAULT_PASSWORD: {{PGADMIN_DEFAULT_PASSWORD}}
    volumes:
      - data:/var/lib/pgadmin

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "PGADMIN_DEFAULT_EMAIL", label: "Email admin", default: Some("admin@example.com"), required: true, secret: false },
            TemplateVar { key: "PGADMIN_DEFAULT_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "photoprism",
        name: "Photoprism",
        description: "Organizador inteligente de fotos com IA",
        category: TemplateCategory::Media,
        default_port: 2342,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: photoprism
      MYSQL_USER: photoprism
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  photoprism:
    image: photoprism/photoprism:latest
    restart: unless-stopped
    ports:
      - "2342"
    environment:
      DB_HOST: db
      DB_NAME: photoprism
      DB_USER: photoprism
      DB_PASSWORD: {{DB_PASSWORD}}
      PHOTOPRISM_ADMIN_PASSWORD: {{PHOTOPRISM_ADMIN_PASSWORD}}
      PHOTOPRISM_SITE_URL: {{PHOTOPRISM_SITE_URL}}
    volumes:
      - originals:/photoprism/originals
      - storage:/photoprism/storage
    depends_on:
      - db

volumes:
  db_data:
  originals:
  storage:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "PHOTOPRISM_ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
            TemplateVar { key: "PHOTOPRISM_SITE_URL", label: "URL do site", default: Some("http://localhost:2342"), required: true, secret: false },
        ],
    },

    Template {
        id: "phpmyadmin",
        name: "phpMyAdmin",
        description: "Gerenciador web para bancos MySQL e MariaDB",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  phpmyadmin:
    image: phpmyadmin:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      PMA_HOST: {{PMA_HOST}}
      MYSQL_ROOT_PASSWORD: {{MYSQL_ROOT_PASSWORD}}
"#,
        variables: &[
            TemplateVar { key: "PMA_HOST", label: "Host MySQL", default: Some("db"), required: true, secret: false },
            TemplateVar { key: "MYSQL_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "plane",
        name: "Plane",
        description: "Sistema moderno de gerenciamento de projetos e sprints",
        category: TemplateCategory::ProjectManagement,
        default_port: 80,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: plane
      POSTGRES_USER: plane
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  plane:
    image: makeplane/plane-space:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DATABASE_URL: postgresql://plane:{{DB_PASSWORD}}@db:5432/plane
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - media:/code/plane-media
    depends_on:
      - db

volumes:
  db_data:
  media:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "prometheus",
        name: "Prometheus",
        description: "Central de monitoramento de séries temporais via scraping",
        category: TemplateCategory::Monitoring,
        default_port: 9090,
        compose: r#"services:
  prometheus:
    image: prom/prometheus:latest
    restart: unless-stopped
    ports:
      - "9090"
    volumes:
      - data:/prometheus
      - config:/etc/prometheus

volumes:
  data:
  config:
"#,
        variables: &[

        ],
    },

    Template {
        id: "pterodactyl",
        name: "Pterodactyl",
        description: "Painel robusto para gerenciamento de servidores de jogos",
        category: TemplateCategory::Gaming,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: pterodactyl
      MYSQL_USER: pterodactyl
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  pterodactyl:
    image: ghcr.io/pterodactyl/panel:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: pterodactyl
      DB_USER: pterodactyl
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
      APP_URL: {{APP_URL}}
    volumes:
      - data:/app/var
      - logs:/app/storage/logs
    depends_on:
      - db

volumes:
  db_data:
  data:
  logs:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_KEY", label: "App Key", default: None, required: true, secret: true },
            TemplateVar { key: "APP_URL", label: "URL da aplicação", default: Some("http://localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "qbittorrent",
        name: "qBittorrent",
        description: "Cliente BitTorrent com interface web nativa",
        category: TemplateCategory::Media,
        default_port: 8080,
        compose: r#"services:
  qbittorrent:
    image: lscr.io/linuxserver/qbittorrent:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      WEBUI_PORT: {{WEBUI_PORT}}
    volumes:
      - config:/config
      - downloads:/downloads

volumes:
  config:
  downloads:
"#,
        variables: &[
            TemplateVar { key: "WEBUI_PORT", label: "Porta WebUI", default: Some("8080"), required: false, secret: false },
        ],
    },

    Template {
        id: "qdrant",
        name: "Qdrant",
        description: "Banco de dados vetorial para busca de similaridade e embeddings",
        category: TemplateCategory::Database,
        default_port: 6333,
        compose: r#"services:
  qdrant:
    image: qdrant/qdrant:latest
    restart: unless-stopped
    ports:
      - "6333"
    volumes:
      - data:/qdrant/storage

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "rabbitmq",
        name: "RabbitMQ",
        description: "Broker de mensageria multi-protocolo para comunicação assíncrona",
        category: TemplateCategory::Database,
        default_port: 5672,
        compose: r#"services:
  rabbitmq:
    image: rabbitmq:management
    restart: unless-stopped
    ports:
      - "5672"
    environment:
      RABBITMQ_DEFAULT_USER: {{RABBITMQ_DEFAULT_USER}}
      RABBITMQ_DEFAULT_PASS: {{RABBITMQ_DEFAULT_PASS}}
    volumes:
      - data:/var/lib/rabbitmq

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "RABBITMQ_DEFAULT_USER", label: "Usuário", default: Some("guest"), required: true, secret: false },
            TemplateVar { key: "RABBITMQ_DEFAULT_PASS", label: "Senha", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "reactive-resume",
        name: "Reactive Resume",
        description: "Gerador moderno de currículos com exportação limpa",
        category: TemplateCategory::DevTools,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: reactive_resume
      POSTGRES_USER: reactive_resume
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  reactive-resume:
    image: amruthpillai/reactive-resume:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://reactive_resume:{{DB_PASSWORD}}@db:5432/reactive_resume
      SECRET_KEY: {{SECRET_KEY}}
      JWT_SECRET: {{JWT_SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "rsshub",
        name: "RSSHub",
        description: "Gerador dinâmico de feeds RSS para milhares de serviços web",
        category: TemplateCategory::DevTools,
        default_port: 1200,
        compose: r#"services:
  rsshub:
    image: diygod/rsshub:latest
    restart: unless-stopped
    ports:
      - "1200"
"#,
        variables: &[

        ],
    },

    Template {
        id: "rustdesk",
        name: "RustDesk",
        description: "Servidor de acesso remoto (Alternativa ao TeamViewer/Anydesk)",
        category: TemplateCategory::Networking,
        default_port: 21115,
        compose: r#"services:
  rustdesk:
    image: rustdesk/rustdesk-server:latest
    restart: unless-stopped
    ports:
      - "21115"
    volumes:
      - data:/root

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "searxng",
        name: "SearXNG",
        description: "Metamecanismo de busca privado sem rastreamento de dados",
        category: TemplateCategory::Networking,
        default_port: 8080,
        compose: r#"services:
  searxng:
    image: searxng/searxng:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - config:/etc/searxng

volumes:
  config:
"#,
        variables: &[

        ],
    },

    Template {
        id: "seafile",
        name: "Seafile",
        description: "Nuvem privada para armazenamento e sincronização de arquivos",
        category: TemplateCategory::Storage,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: seafile
      MYSQL_USER: seafile
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  seafile:
    image: seafileltd/seafile-mc:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: seafile
      DB_USER: seafile
      DB_PASSWORD: {{DB_PASSWORD}}
      SEAFILE_ADMIN_EMAIL: {{SEAFILE_ADMIN_EMAIL}}
      SEAFILE_ADMIN_PASSWORD: {{SEAFILE_ADMIN_PASSWORD}}
      SEAFILE_SERVER_HOSTNAME: {{SEAFILE_SERVER_HOSTNAME}}
    volumes:
      - data:/shared
    depends_on:
      - db

volumes:
  db_data:
  data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SEAFILE_ADMIN_EMAIL", label: "Email admin", default: Some("admin@example.com"), required: true, secret: false },
            TemplateVar { key: "SEAFILE_ADMIN_PASSWORD", label: "Senha admin", default: None, required: true, secret: true },
            TemplateVar { key: "SEAFILE_SERVER_HOSTNAME", label: "Hostname", default: Some("localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "shlink",
        name: "Shlink",
        description: "Encurtador de links corporativo auto-hospedado",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: shlink
      POSTGRES_USER: shlink
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  shlink:
    image: shlinkio/shlink:stable
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://shlink:{{DB_PASSWORD}}@db:5432/shlink
      DEFAULT_DOMAIN: {{DEFAULT_DOMAIN}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "DEFAULT_DOMAIN", label: "Domínio padrão", default: Some("localhost"), required: true, secret: false },
        ],
    },

    Template {
        id: "soketi",
        name: "Soketi",
        description: "Servidor WebSockets ultrarrápido (Compatível com Pusher/Laravel)",
        category: TemplateCategory::DevTools,
        default_port: 6001,
        compose: r#"services:
  soketi:
    image: quay.io/soketi/soketi:latest
    restart: unless-stopped
    ports:
      - "6001"
    environment:
      SOKETI_DEFAULT_APP_ID: {{SOKETI_DEFAULT_APP_ID}}
      SOKETI_DEFAULT_APP_KEY: {{SOKETI_DEFAULT_APP_KEY}}
      SOKETI_DEFAULT_APP_SECRET: {{SOKETI_DEFAULT_APP_SECRET}}
"#,
        variables: &[
            TemplateVar { key: "SOKETI_DEFAULT_APP_ID", label: "App ID", default: Some("app-id"), required: true, secret: false },
            TemplateVar { key: "SOKETI_DEFAULT_APP_KEY", label: "App Key", default: None, required: true, secret: true },
            TemplateVar { key: "SOKETI_DEFAULT_APP_SECRET", label: "App Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "strapi",
        name: "Strapi",
        description: "CMS Headless líder em JavaScript para APIs de conteúdo",
        category: TemplateCategory::Cms,
        default_port: 1337,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: strapi
      POSTGRES_USER: strapi
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  strapi:
    image: strapi/strapi:latest
    restart: unless-stopped
    ports:
      - "1337"
    environment:
      DATABASE_URL: postgresql://strapi:{{DB_PASSWORD}}@db:5432/strapi
      APP_KEYS: {{APP_KEYS}}
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - uploads:/opt/app/public/uploads
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "APP_KEYS", label: "App Keys (4 chaves separadas por vírgula)", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "supabase",
        name: "Supabase",
        description: "Alternativa open-source ao Firebase baseada em PostgreSQL",
        category: TemplateCategory::DevTools,
        default_port: 8000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: supabase
      POSTGRES_USER: supabase
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  supabase:
    image: supabase/supabase-studio:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      DATABASE_URL: postgresql://supabase:{{DB_PASSWORD}}@db:5432/supabase
      JWT_SECRET: {{JWT_SECRET}}
      ANON_KEY: {{ANON_KEY}}
      SERVICE_KEY: {{SERVICE_KEY}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
            TemplateVar { key: "ANON_KEY", label: "Anon Key", default: None, required: true, secret: true },
            TemplateVar { key: "SERVICE_KEY", label: "Service Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "surrealdb",
        name: "SurrealDB",
        description: "Banco de dados multimodel moderno (relacional, grafos, vetorial)",
        category: TemplateCategory::Database,
        default_port: 8000,
        compose: r#"services:
  surrealdb:
    image: surrealdb/surrealdb:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      SURREAL_USER: {{SURREAL_USER}}
      SURREAL_PASS: {{SURREAL_PASS}}
    volumes:
      - data:/mydata

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "SURREAL_USER", label: "Usuário root", default: Some("root"), required: true, secret: false },
            TemplateVar { key: "SURREAL_PASS", label: "Senha root", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "syncthing",
        name: "Syncthing",
        description: "Sincronizador contínuo e descentralizado de diretórios",
        category: TemplateCategory::Backup,
        default_port: 8384,
        compose: r#"services:
  syncthing:
    image: syncthing/syncthing:latest
    restart: unless-stopped
    ports:
      - "8384"
    volumes:
      - data:/var/syncthing

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "trilium",
        name: "Trilium",
        description: "Editor de notas hierárquico para grandes bases de conhecimento",
        category: TemplateCategory::DevTools,
        default_port: 8080,
        compose: r#"services:
  trilium:
    image: zadam/trilium:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - data:/root/trilium-data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "twenty-crm",
        name: "Twenty CRM",
        description: "CRM moderno (Alternativa open-source ao Salesforce)",
        category: TemplateCategory::ProjectManagement,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: twenty_crm
      POSTGRES_USER: twenty_crm
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  twenty-crm:
    image: twentyhq/twenty-front:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://twenty_crm:{{DB_PASSWORD}}@db:5432/twenty_crm
      SECRET: {{SECRET}}
    volumes:
      - server_local_data:/app/packages/twenty-server/.local-storage
    depends_on:
      - db

volumes:
  db_data:
  server_local_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "typebot",
        name: "Typebot",
        description: "Construtor visual de fluxos de conversação e chatbots",
        category: TemplateCategory::Automation,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: typebot
      POSTGRES_USER: typebot
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  typebot:
    image: baptistearno/typebot-builder:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://typebot:{{DB_PASSWORD}}@db:5432/typebot
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      ENCRYPTION_SECRET: {{ENCRYPTION_SECRET}}
      NEXTAUTH_URL: {{NEXTAUTH_URL}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "NEXTAUTH_SECRET", label: "NextAuth Secret", default: None, required: true, secret: true },
            TemplateVar { key: "ENCRYPTION_SECRET", label: "Encryption Secret", default: None, required: true, secret: true },
            TemplateVar { key: "NEXTAUTH_URL", label: "URL da aplicação", default: Some("http://localhost:3000"), required: true, secret: false },
        ],
    },

    Template {
        id: "typesense",
        name: "Typesense",
        description: "Motor de busca rápida e tolerante a falhas para tempo real",
        category: TemplateCategory::DevTools,
        default_port: 8108,
        compose: r#"services:
  typesense:
    image: typesense/typesense:latest
    restart: unless-stopped
    ports:
      - "8108"
    environment:
      TYPESENSE_API_KEY: {{TYPESENSE_API_KEY}}
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "TYPESENSE_API_KEY", label: "API Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "unleash",
        name: "Unleash",
        description: "Plataforma corporativa para gerenciamento de Feature Flags",
        category: TemplateCategory::DevTools,
        default_port: 4242,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: unleash
      POSTGRES_USER: unleash
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  unleash:
    image: unleashorg/unleash-server:latest
    restart: unless-stopped
    ports:
      - "4242"
    environment:
      DATABASE_URL: postgresql://unleash:{{DB_PASSWORD}}@db:5432/unleash
      AUTH_ADMIN_TOKEN: {{AUTH_ADMIN_TOKEN}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "AUTH_ADMIN_TOKEN", label: "Admin Token", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "upsnap",
        name: "Upsnap",
        description: "Dashboard para Wake-on-LAN e monitoramento de dispositivos",
        category: TemplateCategory::Networking,
        default_port: 8090,
        compose: r#"services:
  upsnap:
    image: ghcr.io/seriousm4x/upsnap:latest
    restart: unless-stopped
    ports:
      - "8090"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "valkey",
        name: "Valkey",
        description: "Fork oficial do Redis mantido pela Linux Foundation",
        category: TemplateCategory::Database,
        default_port: 6379,
        compose: r#"services:
  valkey:
    image: valkey/valkey:latest
    restart: unless-stopped
    ports:
      - "6379"
    volumes:
      - data:/data

volumes:
  data:
"#,
        variables: &[

        ],
    },

    Template {
        id: "vault",
        name: "Vault",
        description: "Cofre HashiCorp para gerenciamento estrito de segredos",
        category: TemplateCategory::Security,
        default_port: 8200,
        compose: r#"services:
  vault:
    image: hashicorp/vault:latest
    restart: unless-stopped
    ports:
      - "8200"
    environment:
      VAULT_DEV_ROOT_TOKEN_ID: {{VAULT_DEV_ROOT_TOKEN_ID}}
    volumes:
      - data:/vault/data

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "VAULT_DEV_ROOT_TOKEN_ID", label: "Root Token (modo dev)", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "verdaccio",
        name: "Verdaccio",
        description: "Servidor proxy local e privado para pacotes npm",
        category: TemplateCategory::DevTools,
        default_port: 4873,
        compose: r#"services:
  verdaccio:
    image: verdaccio/verdaccio:latest
    restart: unless-stopped
    ports:
      - "4873"
    volumes:
      - storage:/verdaccio/storage
      - config:/verdaccio/conf

volumes:
  storage:
  config:
"#,
        variables: &[

        ],
    },

    Template {
        id: "vikunja",
        name: "Vikunja",
        description: "Organizador de tarefas com Kanbans e visualizações de Gantt",
        category: TemplateCategory::ProjectManagement,
        default_port: 3456,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: vikunja
      POSTGRES_USER: vikunja
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  vikunja:
    image: vikunja/vikunja:latest
    restart: unless-stopped
    ports:
      - "3456"
    environment:
      DATABASE_URL: postgresql://vikunja:{{DB_PASSWORD}}@db:5432/vikunja
      VIKUNJA_SERVICE_JWT_SECRET: {{VIKUNJA_SERVICE_JWT_SECRET}}
    volumes:
      - files:/app/vikunja/files
    depends_on:
      - db

volumes:
  db_data:
  files:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "VIKUNJA_SERVICE_JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "wallos",
        name: "Wallos",
        description: "Rastreador pessoal de assinaturas mensais e gastos recorrentes",
        category: TemplateCategory::Finance,
        default_port: 8282,
        compose: r#"services:
  wallos:
    image: bellamy9/wallos:latest
    restart: unless-stopped
    ports:
      - "8282"
    volumes:
      - db:/var/www/html/db

volumes:
  db:
"#,
        variables: &[

        ],
    },

    Template {
        id: "wg-easy",
        name: "WG-Easy",
        description: "Interface gráfica simples para servidores VPN WireGuard",
        category: TemplateCategory::Networking,
        default_port: 51821,
        compose: r#"services:
  wg-easy:
    image: ghcr.io/wg-easy/wg-easy:latest
    restart: unless-stopped
    ports:
      - "51821"
    environment:
      WG_HOST: {{WG_HOST}}
      PASSWORD: {{PASSWORD}}
    volumes:
      - data:/etc/wireguard

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "WG_HOST", label: "IP público do servidor", default: None, required: true, secret: false },
            TemplateVar { key: "PASSWORD", label: "Senha da interface web", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "wiki-js",
        name: "Wiki.js",
        description: "Uma das ferramentas mais completas para criação de Wikis",
        category: TemplateCategory::Cms,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: wiki_js
      POSTGRES_USER: wiki_js
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  wiki-js:
    image: ghcr.io/requarks/wiki:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://wiki_js:{{DB_PASSWORD}}@db:5432/wiki_js
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "windmill",
        name: "Windmill",
        description: "Plataforma para workflows internos robustos baseados em scripts",
        category: TemplateCategory::Automation,
        default_port: 8000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: windmill
      POSTGRES_USER: windmill
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  windmill:
    image: ghcr.io/windmill-labs/windmill:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      DATABASE_URL: postgresql://windmill:{{DB_PASSWORD}}@db:5432/windmill
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - worker_dependency_cache:/tmp/windmill/cache
    depends_on:
      - db

volumes:
  db_data:
  worker_dependency_cache:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "JWT_SECRET", label: "JWT Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "xsshunter",
        name: "XSSHunter",
        description: "Ferramenta para pesquisadores focada em Blind XSS",
        category: TemplateCategory::Security,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: xsshunter
      POSTGRES_USER: xsshunter
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  xsshunter:
    image: thehackerish/xsshunter-client:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://xsshunter:{{DB_PASSWORD}}@db:5432/xsshunter
      SECRET: {{SECRET}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "SECRET", label: "Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "yamtrack",
        name: "Yamtrack",
        description: "Gerenciador pessoal de animes e mangás",
        category: TemplateCategory::Media,
        default_port: 8000,
        compose: r#"services:
  yamtrack:
    image: ghcr.io/fcrozetta/yamtrack:latest
    restart: unless-stopped
    ports:
      - "8000"
    environment:
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - data:/app/db

volumes:
  data:
"#,
        variables: &[
            TemplateVar { key: "SECRET_KEY", label: "Secret Key", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "yourls",
        name: "YOURLS",
        description: "Sistema PHP para encurtadores de links privados",
        category: TemplateCategory::DevTools,
        default_port: 80,
        compose: r#"services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: yourls
      MYSQL_USER: yourls
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  yourls:
    image: yourls:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: yourls
      DB_USER: yourls
      DB_PASSWORD: {{DB_PASSWORD}}
      YOURLS_SITE: {{YOURLS_SITE}}
      YOURLS_USER: {{YOURLS_USER}}
      YOURLS_PASS: {{YOURLS_PASS}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_ROOT_PASSWORD", label: "Senha root MySQL", default: None, required: true, secret: true },
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "YOURLS_SITE", label: "URL do site", default: Some("http://localhost"), required: true, secret: false },
            TemplateVar { key: "YOURLS_USER", label: "Usuário admin", default: Some("admin"), required: true, secret: false },
            TemplateVar { key: "YOURLS_PASS", label: "Senha admin", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "yt-dlp-webui",
        name: "yt-dlp-webui",
        description: "Interface gráfica web para o utilitário yt-dlp",
        category: TemplateCategory::Media,
        default_port: 3033,
        compose: r#"services:
  yt-dlp-webui:
    image: ghcr.io/marcopiovanello/yt-dlp-web-ui:latest
    restart: unless-stopped
    ports:
      - "3033"
    volumes:
      - downloads:/downloads

volumes:
  downloads:
"#,
        variables: &[

        ],
    },

    Template {
        id: "zabbix",
        name: "Zabbix",
        description: "Monitor corporativo robusto para redes e servidores",
        category: TemplateCategory::Monitoring,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zabbix
      POSTGRES_USER: zabbix
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zabbix:
    image: zabbix/zabbix-web-nginx-pgsql:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://zabbix:{{DB_PASSWORD}}@db:5432/zabbix
      ZBX_SERVER_HOST: {{ZBX_SERVER_HOST}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "ZBX_SERVER_HOST", label: "Host do Zabbix Server", default: Some("zabbix-server"), required: true, secret: false },
        ],
    },

    Template {
        id: "zipline",
        name: "Zipline",
        description: "Servidor de upload rápido integrado com ShareX",
        category: TemplateCategory::Storage,
        default_port: 3000,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zipline
      POSTGRES_USER: zipline
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zipline:
    image: ghcr.io/diced/zipline:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://zipline:{{DB_PASSWORD}}@db:5432/zipline
      CORE_SECRET: {{CORE_SECRET}}
    volumes:
      - uploads:/zipline/uploads
    depends_on:
      - db

volumes:
  db_data:
  uploads:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "CORE_SECRET", label: "Secret", default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "zitadel",
        name: "Zitadel",
        description: "Provedor de identidade com suporte nativo a multi-tenancy",
        category: TemplateCategory::Security,
        default_port: 8080,
        compose: r#"services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zitadel
      POSTGRES_USER: zitadel
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zitadel:
    image: ghcr.io/zitadel/zitadel:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://zitadel:{{DB_PASSWORD}}@db:5432/zitadel
      ZITADEL_MASTERKEY: {{ZITADEL_MASTERKEY}}
    depends_on:
      - db

volumes:
  db_data:
"#,
        variables: &[
            TemplateVar { key: "DB_PASSWORD", label: "Senha do banco", default: None, required: true, secret: true },
            TemplateVar { key: "ZITADEL_MASTERKEY", label: "Master Key (32 bytes)", default: None, required: true, secret: true },
        ],
    },
];
