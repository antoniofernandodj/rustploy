use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "calibre",
    name: "Calibre",
    description: "Gerenciador e organizador de bibliotecas de e-books",
    category: TemplateCategory::Media,
    default_port: 8080,
    compose: r#"
services:
  calibre:
    image: lscr.io/linuxserver/calibre:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      PASSWORD: {{PASSWORD}}
    volumes:
      - config:/config

volumes:
  config:
"#,
    variables: &[TemplateVar {
        key: "PASSWORD",
        label: "Senha de acesso",
        default: None,
        required: false,
        secret: true,
    }],
};
