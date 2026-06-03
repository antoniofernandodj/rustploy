use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "crowdsec",
    name: "CrowdSec",
    description: "Sistema de segurança colaborativo contra IPs maliciosos",
    category: TemplateCategory::Security,
    default_port: 8080,
    compose: r#"
services:
  crowdsec:
    image: crowdsecurity/crowdsec:latest
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - config:/etc/crowdsec
      - data:/var/lib/crowdsec/data

volumes:
  config:
  data:
"#,
    variables: &[],
};
