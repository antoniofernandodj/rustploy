use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "yourls",
    name: "YOURLS",
    description: "Sistema PHP para encurtadores de links privados",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: yourls
      MYSQL_USER: yourls
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  yourls:
    image: yourls:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: yourls
      DB_USER: yourls
      DB_PASSWORD: {{DB_PASSWORD}}
      YOURLS_SITE: {{YOURLS_SITE}}
      YOURLS_USER: {{YOURLS_USER}}
      YOURLS_PASS: {{YOURLS_PASS}}
    depends_on:
      - db

volumes:
  db_data:
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
            key: "YOURLS_SITE",
            label: "URL do site",
            default: Some("http://localhost"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "YOURLS_USER",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "YOURLS_PASS",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
