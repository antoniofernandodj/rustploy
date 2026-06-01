use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "reactive-resume",
    name: "Reactive Resume",
    description: "Gerador moderno de currículos com exportação limpa",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: reactive_resume
      POSTGRES_USER: reactive_resume
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  reactive-resume:
    image: amruthpillai/reactive-resume:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      DATABASE_URL: postgresql://reactive_resume:{{DB_PASSWORD}}@db:5432/reactive_resume
      SECRET_KEY: {{SECRET_KEY}}
      JWT_SECRET: {{JWT_SECRET}}
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
            key: "SECRET_KEY",
            label: "Secret Key",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
