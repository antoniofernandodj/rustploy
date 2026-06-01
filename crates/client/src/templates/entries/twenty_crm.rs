use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "twenty-crm",
    name: "Twenty CRM",
    description: "CRM moderno (Alternativa open-source ao Salesforce)",
    category: TemplateCategory::ProjectManagement,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: twenty_crm
      POSTGRES_USER: twenty_crm
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  twenty-crm:
    image: twentyhq/twenty-front:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://twenty_crm:{{DB_PASSWORD}}@db:5432/twenty_crm
      SECRET: {{SECRET}}
    volumes:
      - server_local_data:/app/packages/twenty-server/.local-storage
    depends_on:
      - db

volumes:
  db_data:
  server_local_data:
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
            label: "Secret Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
