use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "metube",
    name: "MeTube",
    description: "Downloader web de vídeos do YouTube via yt-dlp",
    category: TemplateCategory::Media,
    default_port: 8081,
    compose: r#"
services:
  metube:
    image: ghcr.io/alexta69/metube:latest
    restart: unless-stopped
    expose:
      - "8081"
    volumes:
      - downloads:/downloads

volumes:
  downloads:
"#,
    variables: &[],
};
