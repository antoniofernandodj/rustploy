use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "excalidraw",
    name: "Excalidraw",
    description: "Quadro branco virtual para esboços e diagramas",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  excalidraw:
    image: excalidraw/excalidraw:latest
    restart: unless-stopped
    expose:
      - "80"
"#,
    variables: &[],
};
