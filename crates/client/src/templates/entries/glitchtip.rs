use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "glitchtip",
    name: "GlitchTip",
    description: "Coletor centralizado de erros (Alternativa ao Sentry)",
    category: TemplateCategory::Monitoring,
    default_port: 8000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: glitchtip
      POSTGRES_USER: glitchtip
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  glitchtip:
    image: glitchtip/glitchtip:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      DATABASE_URL: postgresql://glitchtip:{{DB_PASSWORD}}@db:5432/glitchtip
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - uploads:/code/uploads
    depends_on:
      - db

volumes:
  db_data:
  uploads:
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
