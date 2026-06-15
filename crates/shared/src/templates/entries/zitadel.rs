use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "zitadel",
    name: "Zitadel",
    description: "Provedor de identidade com suporte nativo a multi-tenancy",
    category: TemplateCategory::Security,
    default_port: 8080,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: zitadel
      POSTGRES_USER: zitadel
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  zitadel:
    image: ghcr.io/zitadel/zitadel:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      DATABASE_URL: postgresql://zitadel:{{DB_PASSWORD}}@db:5432/zitadel
      ZITADEL_MASTERKEY: {{ZITADEL_MASTERKEY}}
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
            key: "ZITADEL_MASTERKEY",
            label: "Master Key (32 bytes)",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
