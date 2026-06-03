use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "wiki-js",
    name: "Wiki.js",
    description: "Uma das ferramentas mais completas para criação de Wikis",
    category: TemplateCategory::Cms,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: wiki_js
      POSTGRES_USER: wiki_js
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  wiki-js:
    image: ghcr.io/requarks/wiki:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://wiki_js:{{DB_PASSWORD}}@db:5432/wiki_js
    depends_on:
      - db

volumes:
  db_data:
"#,
    variables: &[TemplateVar {
        key: "DB_PASSWORD",
        label: "Senha do banco",
        default: None,
        required: true,
        secret: true,
    }],
};
