use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "libretranslate",
    name: "LibreTranslate",
    description: "API auto-hospedada de tradução sem dependências externas",
    category: TemplateCategory::Ai,
    default_port: 5000,
    compose: r#"
services:
  libretranslate:
    image: libretranslate/libretranslate:latest
    restart: unless-stopped
    ports:
      - "5000"
    volumes:
      - data:/home/libretranslate/.local/share

volumes:
  data:
"#,
    variables: &[],
};
