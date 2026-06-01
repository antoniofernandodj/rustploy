use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "jellyfin",
    name: "Jellyfin",
    description: "Servidor de mídia e streaming gratuito e open-source",
    category: TemplateCategory::Media,
    default_port: 8096,
    compose: r#"
services:
  jellyfin:
    image: jellyfin/jellyfin:latest
    restart: unless-stopped
    ports:
      - "8096"
    volumes:
      - config:/config
      - cache:/cache

volumes:
  config:
  cache:
"#,
    variables: &[],
};
