use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "emqx",
    name: "EMQX",
    description: "Broker MQTT massivamente escalável para projetos IoT",
    category: TemplateCategory::Networking,
    default_port: 1883,
    compose: r#"
services:
  emqx:
    image: emqx:latest
    restart: unless-stopped
    expose:
      - "1883"
    volumes:
      - data:/opt/emqx/data

volumes:
  data:
"#,
    variables: &[],
};
