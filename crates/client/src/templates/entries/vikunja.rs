use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "vikunja",
    name: "Vikunja",
    description: "Organizador de tarefas com Kanbans e visualizações de Gantt",
    category: TemplateCategory::ProjectManagement,
    default_port: 3456,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: vikunja
      POSTGRES_USER: vikunja
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  vikunja:
    image: vikunja/vikunja:latest
    restart: unless-stopped
    ports:
      - "3456"
    environment:
      DATABASE_URL: postgresql://vikunja:{{DB_PASSWORD}}@db:5432/vikunja
      VIKUNJA_SERVICE_JWT_SECRET: {{VIKUNJA_SERVICE_JWT_SECRET}}
    volumes:
      - files:/app/vikunja/files
    depends_on:
      - db

volumes:
  db_data:
  files:
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
            key: "VIKUNJA_SERVICE_JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
