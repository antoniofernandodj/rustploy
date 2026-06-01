use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "focalboard",
    name: "Focalboard",
    description: "Gerenciador de tarefas Kanban (Alternativa ao Trello/Asana)",
    category: TemplateCategory::ProjectManagement,
    default_port: 8000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: focalboard
      POSTGRES_USER: focalboard
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  focalboard:
    image: mattermost/focalboard:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      DATABASE_URL: postgresql://focalboard:{{DB_PASSWORD}}@db:5432/focalboard
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
