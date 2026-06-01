use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "authentik",
    name: "Authentik",
    description: "Gerenciador de identidade e SSO (OAuth2, SAML, OIDC)",
    category: TemplateCategory::Security,
    default_port: 9000,
    compose: r#"
services:
  postgresql:
    image: postgres:16
    restart: unless-stopped
    environment:
      POSTGRES_DB: authentik
      POSTGRES_USER: authentik
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data

  redis:
    image: redis:alpine
    restart: unless-stopped

  server:
    image: ghcr.io/goauthentik/server:latest
    restart: unless-stopped
    command: server
    ports:
      - "9000"
      - "9443"
    environment:
      AUTHENTIK_REDIS__HOST: redis
      AUTHENTIK_POSTGRESQL__HOST: postgresql
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: {{DB_PASSWORD}}
      AUTHENTIK_POSTGRESQL__NAME: authentik
      AUTHENTIK_SECRET_KEY: {{SECRET_KEY}}
    depends_on:
      - postgresql
      - redis

  worker:
    image: ghcr.io/goauthentik/server:latest
    restart: unless-stopped
    command: worker
    environment:
      AUTHENTIK_REDIS__HOST: redis
      AUTHENTIK_POSTGRESQL__HOST: postgresql
      AUTHENTIK_POSTGRESQL__USER: authentik
      AUTHENTIK_POSTGRESQL__PASSWORD: {{DB_PASSWORD}}
      AUTHENTIK_POSTGRESQL__NAME: authentik
      AUTHENTIK_SECRET_KEY: {{SECRET_KEY}}
    depends_on:
      - postgresql
      - redis

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
            key: "SECRET_KEY",
            label: "Secret Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
