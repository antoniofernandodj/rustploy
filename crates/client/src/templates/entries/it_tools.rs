use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "it-tools",
    name: "IT Tools",
    description: "Coleção de utilitários online essenciais para desenvolvedores",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  it-tools:
    image: corentinth/it-tools:latest
    restart: unless-stopped
    ports:
      - "80"
"#,
    variables: &[],
};
