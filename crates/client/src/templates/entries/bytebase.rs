use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "bytebase",
    name: "Bytebase",
    description: "Ferramenta para controle do ciclo de vida de bancos de dados",
    category: TemplateCategory::DevTools,
    default_port: 5678,
    compose: r#"
services:
  bytebase:
    image: bytebase/bytebase:latest
    restart: unless-stopped
    expose:
      - "5678"
    volumes:
      - data:/var/opt/bytebase

volumes:
  data:
"#,
    variables: &[],
};
