use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "docmost",
    name: "Docmost",
    description: "Wiki colaborativa open-source para equipes",
    category: TemplateCategory::Cms,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: docmost
      POSTGRES_USER: docmost
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  docmost:
    image: docmost/docmost:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://docmost:{{DB_PASSWORD}}@db:5432/docmost
      APP_SECRET: {{APP_SECRET}}
    volumes:
      - storage:/app/data/storage
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
            key: "APP_SECRET",
            label: "App Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
