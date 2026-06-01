use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "invoiceshelf",
    name: "InvoiceShelf",
    description: "Emissor de faturas para profissionais autônomos",
    category: TemplateCategory::Finance,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: invoiceshelf
      MYSQL_USER: invoiceshelf
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  invoiceshelf:
    image: invoiceshelf/invoiceshelf:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: invoiceshelf
      DB_USER: invoiceshelf
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
    volumes:
      - storage:/var/www/html/storage
    depends_on:
      - db

volumes:
  db_data:
  storage:
"#,
    variables: &[
        TemplateVar {
            key: "DB_ROOT_PASSWORD",
            label: "Senha root MySQL",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "APP_KEY",
            label: "App Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
