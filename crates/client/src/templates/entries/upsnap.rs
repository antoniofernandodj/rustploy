use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "upsnap",
    name: "Upsnap",
    description: "Dashboard para Wake-on-LAN e monitoramento de dispositivos",
    category: TemplateCategory::Networking,
    default_port: 8090,
    compose: r#"
services:
  upsnap:
    image: ghcr.io/seriousm4x/upsnap:latest
    restart: unless-stopped
    expose:
      - "8090"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
