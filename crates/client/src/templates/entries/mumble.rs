use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "mumble",
    name: "Mumble",
    description: "Servidor de comunicação de voz com baixíssima latência para jogos",
    category: TemplateCategory::Communication,
    default_port: 64738,
    compose: r#"
services:
  mumble:
    image: mumble/mumble-server:latest
    restart: unless-stopped
    expose:
      - "64738"
    environment:
      MUMBLE_SUPERUSER_PASSWORD: {{MUMBLE_SUPERUSER_PASSWORD}}
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "MUMBLE_SUPERUSER_PASSWORD",
        label: "Senha superuser",
        default: None,
        required: true,
        secret: true,
    }],
};
