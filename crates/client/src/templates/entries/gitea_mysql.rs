use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "gitea-mysql",
    name: "Gitea (MySQL)",
    description: "Servidor Git Gitea com banco de dados MySQL",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: gitea_mysql
      MYSQL_USER: gitea_mysql
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  gitea-mysql:
    image: gitea/gitea:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DB_HOST: db
      DB_NAME: gitea_mysql
      DB_USER: gitea_mysql
      DB_PASSWORD: {{DB_PASSWORD}}
      GITEA__server__DOMAIN: {{GITEA__server__DOMAIN}}
    volumes:
      - data:/data
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
            key: "GITEA__server__DOMAIN",
            label: "Domínio",
            default: Some("localhost"),
            required: true,
            secret: false,
        },
    ],
};
