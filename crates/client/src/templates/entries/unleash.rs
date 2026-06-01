use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "unleash",
    name: "Unleash",
    description: "Plataforma corporativa para gerenciamento de Feature Flags",
    category: TemplateCategory::DevTools,
    default_port: 4242,
    compose: r#"
services:
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
    expose:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "AUTH_ADMIN_TOKEN",
            label: "Admin Token",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
