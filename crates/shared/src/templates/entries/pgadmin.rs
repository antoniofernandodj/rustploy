use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "pgadmin",
    name: "pgAdmin",
    description: "Interface gráfica oficial para gerenciamento de PostgreSQL",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  pgadmin:
    image: dpage/pgadmin4:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      PGADMIN_DEFAULT_EMAIL: {{PGADMIN_DEFAULT_EMAIL}}
      PGADMIN_DEFAULT_PASSWORD: {{PGADMIN_DEFAULT_PASSWORD}}
    volumes:
      - data:/var/lib/pgadmin

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "PGADMIN_DEFAULT_EMAIL",
            label: "Email admin",
            default: Some("admin@example.com"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "PGADMIN_DEFAULT_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
