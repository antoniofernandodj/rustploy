use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "openresty-manager",
    name: "OpenResty Manager",
    description: "Painel para servidores Nginx/OpenResty com SSL",
    category: TemplateCategory::Networking,
    default_port: 81,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: openresty_manager
      MYSQL_USER: openresty_manager
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  openresty-manager:
    image: jc21/nginx-proxy-manager:latest
    restart: unless-stopped
    expose:
      - "81"
    environment:
      DB_HOST: db
      DB_NAME: openresty_manager
      DB_USER: openresty_manager
      DB_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - data:/data
      - letsencrypt:/etc/letsencrypt
    depends_on:
      - db

volumes:
  db_data:
  data:
  letsencrypt:
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
