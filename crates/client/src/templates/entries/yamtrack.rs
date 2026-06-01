use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "yamtrack",
    name: "Yamtrack",
    description: "Gerenciador pessoal de animes e mangás",
    category: TemplateCategory::Media,
    default_port: 8000,
    compose: r#"
services:
  yamtrack:
    image: ghcr.io/fcrozetta/yamtrack:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      SECRET_KEY: {{SECRET_KEY}}
    volumes:
      - data:/app/db

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "SECRET_KEY",
        label: "Secret Key",
        default: None,
        required: true,
        secret: true,
    }],
};
