use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "ghost",
    name: "Ghost",
    description: "Plataforma de blog e newsletter profissional",
    category: TemplateCategory::Cms,
    default_port: 2368,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: ghost
    volumes:
      - db_data:/var/lib/mysql

  ghost:
    image: ghost:latest
    restart: unless-stopped
    ports:
      - "2368"
    environment:
      database__client: mysql
      database__connection__host: db
      database__connection__database: ghost
      database__connection__user: root
      database__connection__password: {{DB_ROOT_PASSWORD}}
      url: http://{{DOMAIN}}
    volumes:
      - ghost_data:/var/lib/ghost/content
    depends_on:
      - db

volumes:
  db_data:
  ghost_data:
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
            key: "DOMAIN",
            label: "Domínio (ex: meusite.com)",
            default: Some("localhost:2368"),
            required: true,
            secret: false,
        },
    ],
};
