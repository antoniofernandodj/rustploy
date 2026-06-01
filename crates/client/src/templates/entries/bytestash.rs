use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "bytestash",
    name: "ByteStash",
    description: "Repositório privado e organizador de trechos de código",
    category: TemplateCategory::DevTools,
    default_port: 5000,
    compose: r#"
services:
  bytestash:
    image: ghcr.io/codeharbour/bytestash:latest
    restart: unless-stopped
    ports:
      - "5000"
    environment:
      JWT_SECRET: {{JWT_SECRET}}
    volumes:
      - data:/app/db

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "JWT_SECRET",
        label: "JWT Secret",
        default: None,
        required: true,
        secret: true,
    }],
};
