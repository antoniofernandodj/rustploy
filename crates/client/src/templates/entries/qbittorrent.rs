use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "qbittorrent",
    name: "qBittorrent",
    description: "Cliente BitTorrent com interface web nativa",
    category: TemplateCategory::Media,
    default_port: 8080,
    compose: r#"
services:
  qbittorrent:
    image: lscr.io/linuxserver/qbittorrent:latest
    restart: unless-stopped
    expose:
      - "8080"
    environment:
      WEBUI_PORT: {{WEBUI_PORT}}
    volumes:
      - config:/config
      - downloads:/downloads

volumes:
  config:
  downloads:
"#,
    variables: &[TemplateVar {
        key: "WEBUI_PORT",
        label: "Porta WebUI",
        default: Some("8080"),
        required: false,
        secret: false,
    }],
};
