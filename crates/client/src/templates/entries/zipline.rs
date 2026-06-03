use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "zipline",
    name: "Zipline",
    description: "Servidor de upload rápido integrado com ShareX",
    category: TemplateCategory::Storage,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zipline
      POSTGRES_USER: zipline
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zipline:
    image: ghcr.io/diced/zipline:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://zipline:{{DB_PASSWORD}}@db:5432/zipline
      CORE_SECRET: {{CORE_SECRET}}
    volumes:
      - uploads:/zipline/uploads
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
            key: "CORE_SECRET",
            label: "Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
