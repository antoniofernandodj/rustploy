use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "activepieces",
    name: "Activepieces",
    description: "Automação no-code (Alternativa ao Zapier)",
    category: TemplateCategory::Automation,
    default_port: 8080,
    compose: r#"
services:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "AP_ENCRYPTION_KEY",
            label: "Chave de criptografia",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "AP_JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
