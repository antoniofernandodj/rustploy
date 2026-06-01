use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kaneo",
    name: "Kaneo",
    description: "Plataforma limpa e simplificada de gerenciamento de projetos",
    category: TemplateCategory::ProjectManagement,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kaneo
      POSTGRES_USER: kaneo
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kaneo:
    image: kaneo/kaneo:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://kaneo:{{DB_PASSWORD}}@db:5432/kaneo
      JWT_SECRET: {{JWT_SECRET}}
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
            key: "JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
