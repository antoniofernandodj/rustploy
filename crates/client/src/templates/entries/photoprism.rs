use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "photoprism",
    name: "Photoprism",
    description: "Organizador inteligente de fotos com IA",
    category: TemplateCategory::Media,
    default_port: 2342,
    compose: r#"
services:
  db:
    image: mysql:8
    restart: unless-stopped
    environment:
      MYSQL_ROOT_PASSWORD: {{DB_ROOT_PASSWORD}}
      MYSQL_DATABASE: photoprism
      MYSQL_USER: photoprism
      MYSQL_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/mysql
  photoprism:
    image: photoprism/photoprism:latest
    restart: unless-stopped
    expose:
      - "2342"
    environment:
      DB_HOST: db
      DB_NAME: photoprism
      DB_USER: photoprism
      DB_PASSWORD: {{DB_PASSWORD}}
      PHOTOPRISM_ADMIN_PASSWORD: {{PHOTOPRISM_ADMIN_PASSWORD}}
      PHOTOPRISM_SITE_URL: {{PHOTOPRISM_SITE_URL}}
    volumes:
      - originals:/photoprism/originals
      - storage:/photoprism/storage
    depends_on:
      - db

volumes:
  db_data:
  originals:
  storage:
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
            key: "PHOTOPRISM_ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "PHOTOPRISM_SITE_URL",
            label: "URL do site",
            default: Some("http://localhost:2342"),
            required: true,
            secret: false,
        },
    ],
};
