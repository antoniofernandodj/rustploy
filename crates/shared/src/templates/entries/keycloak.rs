use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "keycloak",
    name: "Keycloak",
    description: "Provedor robusto de gerenciamento de identidade e autenticação",
    category: TemplateCategory::Security,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: keycloak
      POSTGRES_USER: keycloak
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  keycloak:
    image: quay.io/keycloak/keycloak:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      DATABASE_URL: postgresql://keycloak:{{DB_PASSWORD}}@db:5432/keycloak
      KEYCLOAK_ADMIN: {{KEYCLOAK_ADMIN}}
      KEYCLOAK_ADMIN_PASSWORD: {{KEYCLOAK_ADMIN_PASSWORD}}
    depends_on:
      - db

volumes:
  db_data:
"#,
    variables: &[
        TemplateVar {
            key: "DB_PASSWORD",
            label: "Senha do banco",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "KEYCLOAK_ADMIN",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "KEYCLOAK_ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
