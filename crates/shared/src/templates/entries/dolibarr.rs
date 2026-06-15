use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "dolibarr",
    name: "Dolibarr",
    description: "Pacote ERP e CRM para gestão empresarial",
    category: TemplateCategory::Finance,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: dolibarr
      MYSQL_USER: dolibarr
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  dolibarr:
    image: dolibarr/dolibarr:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: dolibarr
      DB_USER: dolibarr
      DB_PASSWORD: {{DB_PASSWORD}}
      DOLI_ADMIN_LOGIN: {{DOLI_ADMIN_LOGIN}}
      DOLI_ADMIN_PASSWORD: {{DOLI_ADMIN_PASSWORD}}
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
            key: "DOLI_ADMIN_LOGIN",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "DOLI_ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
