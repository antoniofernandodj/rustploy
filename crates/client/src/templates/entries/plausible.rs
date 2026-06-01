use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
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
        TemplateVar {
            key: "DOMAIN",
            label: "Domínio base",
            default: Some("localhost:8000"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SECRET_KEY_BASE",
            label: "Secret Key (64 chars)",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
