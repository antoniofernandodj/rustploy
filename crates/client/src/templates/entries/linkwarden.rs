use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "linkwarden",
    name: "Linkwarden",
    description: "Gerenciador de links focado em arquivamento de páginas web",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: linkwarden
      POSTGRES_USER: linkwarden
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  linkwarden:
    image: ghcr.io/linkwarden/linkwarden:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://linkwarden:{{DB_PASSWORD}}@db:5432/linkwarden
      NEXTAUTH_SECRET: {{NEXTAUTH_SECRET}}
      NEXTAUTH_URL: {{NEXTAUTH_URL}}
    volumes:
      - data:/data/data
    depends_on:
      - db

volumes:
  db_data:
  data:
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
            key: "NEXTAUTH_SECRET",
            label: "NextAuth Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "NEXTAUTH_URL",
            label: "URL da aplicação",
            default: Some("http://localhost:3000"),
            required: true,
            secret: false,
        },
    ],
};
