use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "pterodactyl",
    name: "Pterodactyl",
    description: "Painel robusto para gerenciamento de servidores de jogos",
    category: TemplateCategory::Gaming,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: pterodactyl
      MYSQL_USER: pterodactyl
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  pterodactyl:
    image: ghcr.io/pterodactyl/panel:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: pterodactyl
      DB_USER: pterodactyl
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
      APP_URL: {{APP_URL}}
    volumes:
      - data:/app/var
      - logs:/app/storage/logs
    depends_on:
      - db

volumes:
  db_data:
  data:
  logs:
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
        TemplateVar {
            key: "APP_URL",
            label: "URL da aplicação",
            default: Some("http://localhost"),
            required: true,
            secret: false,
        },
    ],
};
