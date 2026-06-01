use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "homeassistant",
    name: "Home Assistant",
    description: "Ecossistema open-source definitivo para automação residencial",
    category: TemplateCategory::Networking,
    default_port: 8123,
    compose: r#"
services:
  homeassistant:
    image: homeassistant/home-assistant:latest
    restart: unless-stopped
    ports:
      - "8123"
    volumes:
      - config:/config

volumes:
  config:
"#,
    variables: &[],
};
