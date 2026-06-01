use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "cloudflared",
    name: "Cloudflared",
    description: "Daemon para conectar serviços locais via Cloudflare Tunnel",
    category: TemplateCategory::Networking,
    default_port: 0,
    compose: r#"
services:
  cloudflared:
    image: cloudflare/cloudflared:latest
    restart: unless-stopped
    environment:
      TUNNEL_TOKEN: {{TUNNEL_TOKEN}}
"#,
    variables: &[TemplateVar {
        key: "TUNNEL_TOKEN",
        label: "Token do Tunnel",
        default: None,
        required: true,
        secret: true,
    }],
};
