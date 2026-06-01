use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "verdaccio",
    name: "Verdaccio",
    description: "Servidor proxy local e privado para pacotes npm",
    category: TemplateCategory::DevTools,
    default_port: 4873,
    compose: r#"
services:
  verdaccio:
    image: verdaccio/verdaccio:latest
    restart: unless-stopped
    expose:
      - "4873"
    volumes:
      - storage:/verdaccio/storage
      - config:/verdaccio/conf

volumes:
  storage:
  config:
"#,
    variables: &[],
};
