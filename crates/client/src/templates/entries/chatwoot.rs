use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "chatwoot",
    name: "Chatwoot",
    description: "Plataforma de atendimento omnichannel (Live Chat, WhatsApp)",
    category: TemplateCategory::Communication,
    default_port: 3000,
    compose: r#"
services:
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
    expose:
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
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SECRET_KEY_BASE",
            label: "Secret Key Base",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
