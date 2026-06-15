use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "searxng",
    name: "SearXNG",
    description: "Metamecanismo de busca privado sem rastreamento de dados",
    category: TemplateCategory::Networking,
    default_port: 8080,
    compose: r#"
services:
  searxng:
    image: searxng/searxng:latest
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - config:/etc/searxng

volumes:
  config:
"#,
    variables: &[],
};
