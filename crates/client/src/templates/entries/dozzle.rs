use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "dozzle",
    name: "Dozzle",
    description: "Visualizador em tempo real de logs Docker",
    category: TemplateCategory::Monitoring,
    default_port: 8080,
    compose: r#"
services:
  dozzle:
    image: amir20/dozzle:latest
    restart: unless-stopped
    ports:
      - "8080"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
"#,
    variables: &[],
};
