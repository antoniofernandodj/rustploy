use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "supabase",
    name: "Supabase",
    description: "Alternativa open-source ao Firebase baseada em PostgreSQL",
    category: TemplateCategory::DevTools,
    default_port: 8000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: supabase
      POSTGRES_USER: supabase
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
  supabase:
    image: supabase/supabase-studio:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      DATABASE_URL: postgresql://supabase:{{DB_PASSWORD}}@db:5432/supabase
      JWT_SECRET: {{JWT_SECRET}}
      ANON_KEY: {{ANON_KEY}}
      SERVICE_KEY: {{SERVICE_KEY}}
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
            key: "JWT_SECRET",
            label: "JWT Secret",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "ANON_KEY",
            label: "Anon Key",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SERVICE_KEY",
            label: "Service Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
