use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
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
    expose:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "APP_SECRET",
            label: "App Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
