use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "logto",
    name: "Logto",
    description: "Plataforma CIAM moderna para autenticação de clientes",
    category: TemplateCategory::Security,
    default_port: 3001,
    compose: r#"
services:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "TRUST_PROXY_HEADER",
            label: "Trust proxy header",
            default: Some("1"),
            required: false,
            secret: false,
        },
    ],
};
