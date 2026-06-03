use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "hoppscotch",
    name: "Hoppscotch",
    description: "Suíte completa de testes de API (Alternativa ao Postman)",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  db:
    image: postgres:15
    restart: unless-stopped
    environment:
      POSTGRES_DB: hoppscotch
      POSTGRES_USER: hoppscotch
      POSTGRES_PASSWORD: {{DB_PASSWORD}}
    volumes:
      - db_data:/var/lib/postgresql/data
    healthcheck:
      test: ["CMD-SHELL", "pg_isready -U hoppscotch"]
      interval: 5s
      timeout: 5s
      retries: 5

  hoppscotch:
    image: hoppscotch/hoppscotch:latest
    restart: unless-stopped
    expose:
      - "3000"
    command: sh -c "cd /dist/backend && npx prisma migrate deploy && exec node /usr/src/app/aio_run.mjs"
    environment:
      PORT: 3000
      DATABASE_URL: postgresql://hoppscotch:{{DB_PASSWORD}}@db:5432/hoppscotch
      JWT_SECRET: {{JWT_SECRET}}
      WHITELISTED_ORIGINS: "*"
      VITE_ALLOWED_HOSTS: "*"
    depends_on:
      db:
        condition: service_healthy

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
    ],
};
