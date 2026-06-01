use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "typebot",
    name: "Typebot",
    description: "Construtor visual de fluxos de conversação e chatbots",
    category: TemplateCategory::Automation,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: typebot
      POSTGRES_USER: typebot
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  typebot:
    image: baptistearno/typebot-builder:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://typebot:{{DB_PASSWORD}}@db:5432/typebot
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      ENCRYPTION_SECRET: {{ENCRYPTION_SECRET}}
      NEXTAUTH_URL: {{NEXTAUTH_URL}}
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
            key: "NEXTAUTH_SECRET",
            label: "NextAuth Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "ENCRYPTION_SECRET",
            label: "Encryption Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "NEXTAUTH_URL",
            label: "URL da aplicação",
            default: Some("http://localhost:3000"),
            required: true,
            secret: false,
        },
    ],
};
