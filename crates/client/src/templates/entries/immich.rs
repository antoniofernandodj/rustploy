use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
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
    variables: &[TemplateVar {
        key: "DB_PASSWORD",
        label: "Senha do banco",
        default: None,
        required: true,
        secret: true,
    }],
};
