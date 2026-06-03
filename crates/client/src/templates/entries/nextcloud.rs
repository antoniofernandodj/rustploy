use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "nextcloud",
    name: "Nextcloud",
    description: "Suite completa de produtividade na nuvem (Postgres)",
    category: TemplateCategory::Storage,
    default_port: 80,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: nextcloud
      POSTGRES_USER: nextcloud
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  nextcloud:
    image: nextcloud:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      POSTGRES_HOST: db
      POSTGRES_DB: nextcloud
      POSTGRES_USER: nextcloud
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
      NEXTCLOUD_ADMIN_USER: {{ADMIN_USER}}
      NEXTCLOUD_ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - nc_data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  nc_data:
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
            key: "ADMIN_USER",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
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
