use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "budibase",
    name: "Budibase",
    description: "Plataforma low-code para criação de ferramentas internas",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: budibase
      POSTGRES_USER: budibase
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  budibase:
    image: budibase/budibase:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DATABASE_URL: postgresql://budibase:{{DB_PASSWORD}}@db:5432/budibase
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
