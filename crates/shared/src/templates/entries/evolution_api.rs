use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "evolution-api",
    name: "Evolution API",
    description: "API de WhatsApp focada em automação para empresas",
    category: TemplateCategory::Automation,
    default_port: 8080,
    compose: r#"
services:
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
    expose:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "AUTHENTICATION_API_KEY",
            label: "API Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
