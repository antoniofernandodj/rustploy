use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "memos",
    name: "Memos",
    description: "Central de notas rápidas focada em privacidade",
    category: TemplateCategory::DevTools,
    default_port: 5230,
    compose: r#"
services:
  memos:
    image: neosmemo/memos:stable
    restart: unless-stopped
    ports:
      - "5230"
    volumes:
      - data:/.memos

volumes:
  data:
"#,
    variables: &[],
};
