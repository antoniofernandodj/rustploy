use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "emby",
    name: "Emby",
    description: "Servidor de mídia privado para streaming de filmes e músicas",
    category: TemplateCategory::Media,
    default_port: 8096,
    compose: r#"
services:
  emby:
    image: emby/embyserver:latest
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
