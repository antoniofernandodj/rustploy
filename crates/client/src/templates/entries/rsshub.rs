use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "rsshub",
    name: "RSSHub",
    description: "Gerador dinâmico de feeds RSS para milhares de serviços web",
    category: TemplateCategory::DevTools,
    default_port: 1200,
    compose: r#"
services:
  rsshub:
    image: diygod/rsshub:latest
    restart: unless-stopped
    ports:
      - "1200"
"#,
    variables: &[],
};
