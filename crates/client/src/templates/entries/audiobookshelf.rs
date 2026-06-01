use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "audiobookshelf",
    name: "Audiobookshelf",
    description: "Servidor de mídia para audiolivros e podcasts",
    category: TemplateCategory::Media,
    default_port: 13378,
    compose: r#"
services:
  audiobookshelf:
    image: ghcr.io/advplyr/audiobookshelf:latest
    restart: unless-stopped
    expose:
      - "13378"
    volumes:
      - config:/config
      - metadata:/metadata
      - audiobooks:/audiobooks

volumes:
  config:
  metadata:
  audiobooks:
"#,
    variables: &[],
};
