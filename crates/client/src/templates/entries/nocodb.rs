use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "nocodb",
    name: "NocoDB",
    description: "Transforma bancos relacionais em interface estilo Airtable",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: nocodb
      POSTGRES_USER: nocodb
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  nocodb:
    image: nocodb/nocodb:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://nocodb:{{DB_PASSWORD}}@db:5432/nocodb
      NC_AUTH_JWT_SECRET: {{NC_AUTH_JWT_SECRET}}
    volumes:
      - data:/usr/app/data
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
            key: "NC_AUTH_JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
