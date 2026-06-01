use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "botpress",
    name: "Botpress",
    description: "Plataforma para criação de agentes de IA conversacionais",
    category: TemplateCategory::Ai,
    default_port: 3000,
    compose: r#"
services:
  botpress:
    image: botpress/server:latest
    restart: unless-stopped
    expose:
      - "3000"
    volumes:
      - data:/botpress/data

volumes:
  data:
"#,
    variables: &[],
};
