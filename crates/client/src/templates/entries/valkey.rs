use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "valkey",
    name: "Valkey",
    description: "Fork oficial do Redis mantido pela Linux Foundation",
    category: TemplateCategory::Database,
    default_port: 6379,
    compose: r#"
services:
  valkey:
    image: valkey/valkey:latest
    restart: unless-stopped
    expose:
      - "6379"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
