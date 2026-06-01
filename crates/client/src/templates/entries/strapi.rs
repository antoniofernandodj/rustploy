use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "strapi",
    name: "Strapi",
    description: "CMS Headless líder em JavaScript para APIs de conteúdo",
    category: TemplateCategory::Cms,
    default_port: 1337,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: strapi
      POSTGRES_USER: strapi
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  strapi:
    image: strapi/strapi:latest
    restart: unless-stopped
    expose:
      - "1337"
    environment:
      DATABASE_URL: postgresql://strapi:{{DB_PASSWORD}}@db:5432/strapi
      APP_KEYS: {{APP_KEYS}}
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - uploads:/opt/app/public/uploads
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
            key: "APP_KEYS",
            label: "App Keys (4 chaves separadas por vírgula)",
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
