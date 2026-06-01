use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "mealie",
    name: "Mealie",
    description: "Gerenciador de receitas com importação automática de sites",
    category: TemplateCategory::DevTools,
    default_port: 9000,
    compose: r#"
services:
  mealie:
    image: ghcr.io/mealie-recipes/mealie:latest
    restart: unless-stopped
    expose:
      - "9000"
    environment:
      DEFAULT_EMAIL: {{DEFAULT_EMAIL}}
      DEFAULT_PASSWORD: {{DEFAULT_PASSWORD}}
    volumes:
      - data:/app/data

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "DEFAULT_EMAIL",
            label: "Email admin",
            default: Some("admin@example.com"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "DEFAULT_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
