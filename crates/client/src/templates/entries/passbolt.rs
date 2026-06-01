use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "passbolt",
    name: "Passbolt",
    description: "Gerenciador de senhas open-source para equipes técnicas",
    category: TemplateCategory::Security,
    default_port: 80,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: passbolt
      MYSQL_USER: passbolt
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  passbolt:
    image: passbolt/passbolt:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      DB_HOST: db
      DB_NAME: passbolt
      DB_USER: passbolt
      DB_PASSWORD: {{DB_PASSWORD}}
      APP_FULL_BASE_URL: {{APP_FULL_BASE_URL}}
    volumes:
      - gpg:/etc/passbolt/gpg
      - jwt:/etc/passbolt/jwt
    depends_on:
      - db

volumes:
  db_data:
  gpg:
  jwt:
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
            key: "APP_FULL_BASE_URL",
            label: "URL base",
            default: Some("https://localhost"),
            required: true,
            secret: false,
        },
    ],
};
