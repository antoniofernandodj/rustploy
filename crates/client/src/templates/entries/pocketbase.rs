use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "pocketbase",
    name: "PocketBase",
    description: "Backend completo em arquivo único com SQLite e realtime",
    category: TemplateCategory::DevTools,
    default_port: 8090,
    compose: r#"
services:
  pocketbase:
    image: ghcr.io/muchobien/pocketbase:latest
    restart: unless-stopped
    expose:
      - "8090"
    volumes:
      - pb_data:/pb_data

volumes:
  pb_data:
"#,
    variables: &[],
};
