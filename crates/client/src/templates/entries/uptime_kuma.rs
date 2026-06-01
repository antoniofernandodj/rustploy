use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "uptime-kuma",
    name: "Uptime Kuma",
    description: "Monitor visual de uptime com alertas",
    category: TemplateCategory::Monitoring,
    default_port: 3001,
    compose: r#"
services:
  uptime-kuma:
    image: louislam/uptime-kuma:latest
    restart: unless-stopped
    expose:
      - "3001"
    volumes:
      - uptime_data:/app/data

volumes:
  uptime_data:
"#,
    variables: &[],
};
