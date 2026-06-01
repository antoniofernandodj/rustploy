use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "yt-dlp-webui",
    name: "yt-dlp-webui",
    description: "Interface gráfica web para o utilitário yt-dlp",
    category: TemplateCategory::Media,
    default_port: 3033,
    compose: r#"
services:
  yt-dlp-webui:
    image: ghcr.io/marcopiovanello/yt-dlp-web-ui:latest
    restart: unless-stopped
    expose:
      - "3033"
    volumes:
      - downloads:/downloads

volumes:
  downloads:
"#,
    variables: &[],
};
