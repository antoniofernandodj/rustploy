use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "directus",
    name: "Directus",
    description: "CMS Headless e wrapper de APIs para SQL",
    category: TemplateCategory::Cms,
    default_port: 8055,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: directus
      POSTGRES_USER: directus
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  directus:
    image: directus/directus:latest
    restart: unless-stopped
    expose:
      - "8055"
    environment:
      DATABASE_URL: postgresql://directus:{{DB_PASSWORD}}@db:5432/directus
      SECRET: {{SECRET}}
      ADMIN_EMAIL: {{ADMIN_EMAIL}}
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - uploads:/directus/uploads
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
            key: "SECRET",
            label: "Chave secreta",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "ADMIN_EMAIL",
            label: "Email admin",
            default: Some("admin@example.com"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
