use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "baserow",
    name: "Baserow",
    description: "Banco de dados relacional com interface de planilha (Alternativa ao Airtable)",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
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
    expose:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SECRET_KEY",
            label: "Secret Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
