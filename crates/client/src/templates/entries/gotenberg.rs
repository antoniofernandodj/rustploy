use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "gotenberg",
    name: "Gotenberg",
    description: "API escalável para conversões de arquivos para PDF",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  gotenberg:
    image: gotenberg/gotenberg:latest
    restart: unless-stopped
    ports:
      - "3000"
"#,
    variables: &[],
};
