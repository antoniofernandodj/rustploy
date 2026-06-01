use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "opengist",
    name: "OpenGist",
    description: "Alternativa ao GitHub Gist para trechos de código",
    category: TemplateCategory::DevTools,
    default_port: 6157,
    compose: r#"
services:
  opengist:
    image: ghcr.io/thomiceli/opengist:latest
    restart: unless-stopped
    expose:
      - "6157"
    volumes:
      - data:/opengist

volumes:
  data:
"#,
    variables: &[],
};
