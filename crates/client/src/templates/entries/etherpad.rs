use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "etherpad",
    name: "Etherpad",
    description: "Editor de texto colaborativo multiusuário em tempo real",
    category: TemplateCategory::DevTools,
    default_port: 9001,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: etherpad
      POSTGRES_USER: etherpad
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  etherpad:
    image: etherpad/etherpad:latest
    restart: unless-stopped
    expose:
      - "9001"
    environment:
      DATABASE_URL: postgresql://etherpad:{{DB_PASSWORD}}@db:5432/etherpad
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
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
            key: "ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
