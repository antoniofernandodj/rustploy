use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "wg-easy",
    name: "WG-Easy",
    description: "Interface gráfica simples para servidores VPN WireGuard",
    category: TemplateCategory::Networking,
    default_port: 51821,
    compose: r#"
services:
  wg-easy:
    image: ghcr.io/wg-easy/wg-easy:latest
    restart: unless-stopped
    ports:
      - "51821"
    environment:
      WG_HOST: {{WG_HOST}}
      PASSWORD: {{PASSWORD}}
    volumes:
      - data:/etc/wireguard

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "WG_HOST",
            label: "IP público do servidor",
            default: None,
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "PASSWORD",
            label: "Senha da interface web",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
