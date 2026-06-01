use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "outline",
    name: "Outline",
    description: "Base de conhecimento corporativa moderna para equipes ágeis",
    category: TemplateCategory::Cms,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: outline
      POSTGRES_USER: outline
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  outline:
    image: outlinewiki/outline:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://outline:{{DB_PASSWORD}}@db:5432/outline
      SECRET_KEY: {{SECRET_KEY}}
      UTILS_SECRET: {{UTILS_SECRET}}
    volumes:
      - data:/var/lib/outline/data
    depends_on:
      - db

volumes:
  db_data:
  data:
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
            label: "Secret Key (32 hex chars)",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "UTILS_SECRET",
            label: "Utils Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
