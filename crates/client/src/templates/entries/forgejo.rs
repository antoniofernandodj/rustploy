use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "forgejo",
    name: "Forgejo",
    description: "Plataforma leve para hospedagem de código Git",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  forgejo:
    image: codeberg.org/forgejo/forgejo:latest
    restart: unless-stopped
    expose:
      - "3000"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
