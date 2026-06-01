use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "plane",
    name: "Plane",
    description: "Sistema moderno de gerenciamento de projetos e sprints",
    category: TemplateCategory::ProjectManagement,
    default_port: 80,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: plane
      POSTGRES_USER: plane
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  plane:
    image: makeplane/plane-space:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      DATABASE_URL: postgresql://plane:{{DB_PASSWORD}}@db:5432/plane
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - media:/code/plane-media
    depends_on:
      - db

volumes:
  db_data:
  media:
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
