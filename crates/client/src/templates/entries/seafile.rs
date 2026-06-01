use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "seafile",
    name: "Seafile",
    description: "Nuvem privada para armazenamento e sincronização de arquivos",
    category: TemplateCategory::Storage,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: seafile
      MYSQL_USER: seafile
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  seafile:
    image: seafileltd/seafile-mc:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: seafile
      DB_USER: seafile
      DB_PASSWORD: {{DB_PASSWORD}}
      SEAFILE_ADMIN_EMAIL: {{SEAFILE_ADMIN_EMAIL}}
      SEAFILE_ADMIN_PASSWORD: {{SEAFILE_ADMIN_PASSWORD}}
      SEAFILE_SERVER_HOSTNAME: {{SEAFILE_SERVER_HOSTNAME}}
    volumes:
      - data:/shared
    depends_on:
      - db

volumes:
  db_data:
  data:
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
            key: "SEAFILE_ADMIN_EMAIL",
            label: "Email admin",
            default: Some("admin@example.com"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "SEAFILE_ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SEAFILE_SERVER_HOSTNAME",
            label: "Hostname",
            default: Some("localhost"),
            required: true,
            secret: false,
        },
    ],
};
