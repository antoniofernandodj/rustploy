use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "onedev",
    name: "OneDev",
    description: "Servidor Git com quadros Kanban e esteiras nativas de CI/CD",
    category: TemplateCategory::DevTools,
    default_port: 6610,
    compose: r#"
services:
  onedev:
    image: 1dev/server:latest
    restart: unless-stopped
    expose:
      - "6610"
    volumes:
      - data:/opt/onedev

volumes:
  data:
"#,
    variables: &[],
};
