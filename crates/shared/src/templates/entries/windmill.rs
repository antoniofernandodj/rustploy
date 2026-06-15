use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "windmill",
    name: "Windmill",
    description: "Plataforma para workflows internos robustos baseados em scripts",
    category: TemplateCategory::Automation,
    default_port: 8000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: windmill
      POSTGRES_USER: windmill
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  windmill:
    image: ghcr.io/windmill-labs/windmill:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      DATABASE_URL: postgresql://windmill:{{DB_PASSWORD}}@db:5432/windmill
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - worker_dependency_cache:/tmp/windmill/cache
    depends_on:
      - db

volumes:
  db_data:
  worker_dependency_cache:
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
