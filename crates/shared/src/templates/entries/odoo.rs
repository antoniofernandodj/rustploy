use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "odoo",
    name: "Odoo",
    description: "Sistema modular ERP open-source para gestão de negócios globais",
    category: TemplateCategory::Finance,
    default_port: 8069,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: odoo
      POSTGRES_USER: odoo
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  odoo:
    image: odoo:latest
    restart: unless-stopped
    expose:
      - "8069"
    environment:
      DATABASE_URL: postgresql://odoo:{{DB_PASSWORD}}@db:5432/odoo
    volumes:
      - data:/var/lib/odoo
      - addons:/mnt/extra-addons
    depends_on:
      - db

volumes:
  db_data:
  data:
  addons:
"#,
    variables: &[TemplateVar {
        key: "DB_PASSWORD",
        label: "Senha do banco",
        default: None,
        required: true,
        secret: true,
    }],
};
