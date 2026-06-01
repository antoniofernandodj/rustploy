use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "shlink",
    name: "Shlink",
    description: "Encurtador de links corporativo auto-hospedado",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: shlink
      POSTGRES_USER: shlink
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  shlink:
    image: shlinkio/shlink:stable
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      DATABASE_URL: postgresql://shlink:{{DB_PASSWORD}}@db:5432/shlink
      DEFAULT_DOMAIN: {{DEFAULT_DOMAIN}}
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
            key: "DEFAULT_DOMAIN",
            label: "Domínio padrão",
            default: Some("localhost"),
            required: true,
            secret: false,
        },
    ],
};
