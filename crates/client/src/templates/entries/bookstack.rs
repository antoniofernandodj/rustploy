use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "bookstack",
    name: "BookStack",
    description: "Plataforma wiki para documentações corporativas",
    category: TemplateCategory::Cms,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: bookstack
      MYSQL_USER: bookstack
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  bookstack:
    image: lscr.io/linuxserver/bookstack:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: bookstack
      DB_USER: bookstack
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_KEY: {{APP_KEY}}
      APP_URL: {{APP_URL}}
    volumes:
      - uploads:/config
    depends_on:
      - db

volumes:
  db_data:
  uploads:
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
