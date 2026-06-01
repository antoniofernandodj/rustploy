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
        compose: r#"services:
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
            TemplateVar { key: "DB_PASSWORD",      label: "Senha do banco",   default: None, required: true, secret: true },
        ],
    },

    Template {
        id: "ghost",
        name: "Ghost",
        description: "Plataforma de blog e newsletter profissional",
        category: TemplateCategory::Cms,
        default_port: 2368,
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
        compose: r#"services:
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
];
