use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "trilium",
    name: "Trilium",
    description: "Editor de notas hierárquico para grandes bases de conhecimento",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  trilium:
    image: zadam/trilium:latest
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - data:/root/trilium-data

volumes:
  data:
"#,
    variables: &[],
};
