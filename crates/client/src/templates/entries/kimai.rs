use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "kimai",
    name: "Kimai",
    description: "Sistema multiusuário para controle de horas trabalhadas",
    category: TemplateCategory::Finance,
    default_port: 8001,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: kimai
      MYSQL_USER: kimai
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  kimai:
    image: kimai/kimai2:apache
    restart: unless-stopped
    ports:
      - "8001"
    environment:
      DB_HOST: db
      DB_NAME: kimai
      DB_USER: kimai
      DB_PASSWORD: {{DB_PASSWORD}}
      ADMINMAIL: {{ADMINMAIL}}
      ADMINPASS: {{ADMINPASS}}
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
            key: "ADMINMAIL",
            label: "Email admin",
            default: Some("admin@example.com"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "ADMINPASS",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
