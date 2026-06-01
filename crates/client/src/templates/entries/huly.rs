use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "huly",
    name: "Huly",
    description: "Gerenciador de projetos (Alternativa ao Jira/Linear/Slack)",
    category: TemplateCategory::ProjectManagement,
    default_port: 8083,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: huly
      POSTGRES_USER: huly
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  huly:
    image: hardcoreeng/huly:latest
    restart: unless-stopped
    expose:
      - "8083"
    environment:
      DATABASE_URL: postgresql://huly:{{DB_PASSWORD}}@db:5432/huly
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
