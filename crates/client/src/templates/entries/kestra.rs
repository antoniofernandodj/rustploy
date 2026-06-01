use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kestra",
    name: "Kestra",
    description: "Orquestrador declarativo de fluxos de dados e negócios",
    category: TemplateCategory::Automation,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: kestra
      POSTGRES_USER: kestra
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  kestra:
    image: kestra/kestra:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      DATABASE_URL: postgresql://kestra:{{DB_PASSWORD}}@db:5432/kestra
    volumes:
      - storage:/app/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
    variables: &[TemplateVar {
        key: "DB_PASSWORD",
        label: "Senha do banco",
        default: None,
        required: true,
        secret: true,
    }],
};
