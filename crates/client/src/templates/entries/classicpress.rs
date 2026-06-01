use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "classicpress",
    name: "ClassicPress",
    description: "Fork estável do WordPress com editor clássico",
    category: TemplateCategory::Cms,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: classicpress
      MYSQL_USER: classicpress
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql

  classicpress:
    image: wordpress:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: classicpress
      WORDPRESS_DB_PASSWORD: {{DB_PASSWORD}}
      WORDPRESS_DB_NAME: classicpress
    volumes:
      - cp_data:/var/www/html
    depends_on:
      - db

volumes:
  db_data:
  cp_data:
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
    ],
};
