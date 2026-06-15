use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "grimoire",
    name: "Grimoire",
    description: "Organizador e salvador de favoritos (bookmarks) ultra veloz",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: grimoire
      POSTGRES_USER: grimoire
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  grimoire:
    image: ghcr.io/goniszewski/grimoire:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      DATABASE_URL: postgresql://grimoire:{{DB_PASSWORD}}@db:5432/grimoire
      SECRET: {{SECRET}}
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
            key: "SECRET",
            label: "Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
