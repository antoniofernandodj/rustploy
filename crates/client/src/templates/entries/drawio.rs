use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "drawio",
    name: "draw.io",
    description: "Ferramenta para desenho de diagramas e quadros brancos",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  drawio:
    image: jgraph/drawio:latest
    restart: unless-stopped
    ports:
      - "8080"
"#,
    variables: &[],
};
