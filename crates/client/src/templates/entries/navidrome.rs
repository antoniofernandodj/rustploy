use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "navidrome",
    name: "Navidrome",
    description: "Servidor leve de streaming de áudio compatível com Subsonic",
    category: TemplateCategory::Media,
    default_port: 4533,
    compose: r#"
services:
  navidrome:
    image: deluan/navidrome:latest
    restart: unless-stopped
    expose:
      - "4533"
    volumes:
      - data:/data
      - music:/music

volumes:
  data:
  music:
"#,
    variables: &[],
};
