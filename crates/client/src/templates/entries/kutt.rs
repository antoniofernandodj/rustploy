use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kutt",
    name: "Kutt",
    description: "Encurtador de URLs moderno com analytics e domínios customizados",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kutt
      POSTGRES_USER: kutt
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kutt:
    image: kutt/kutt:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://kutt:{{DB_PASSWORD}}@db:5432/kutt
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
