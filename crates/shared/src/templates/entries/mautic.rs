use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "mautic",
    name: "Mautic",
    description: "Sistema completo de automação de marketing digital",
    category: TemplateCategory::Automation,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: mautic
      MYSQL_USER: mautic
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  mautic:
    image: mautic/mautic:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: mautic
      DB_USER: mautic
      DB_PASSWORD: {{DB_PASSWORD}}
      MAUTIC_ADMIN_USERNAME: {{MAUTIC_ADMIN_USERNAME}}
      MAUTIC_ADMIN_PASSWORD: {{MAUTIC_ADMIN_PASSWORD}}
    volumes:
      - data:/var/www/html
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
            key: "MAUTIC_ADMIN_USERNAME",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "MAUTIC_ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
