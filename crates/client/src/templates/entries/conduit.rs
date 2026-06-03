use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "conduit",
    name: "Conduit",
    description: "Servidor de chat Matrix ultrarrápido escrito em Rust",
    category: TemplateCategory::Communication,
    default_port: 6167,
    compose: r#"
services:
  conduit:
    image: registry.gitlab.com/famedly/conduit:latest
    restart: unless-stopped
    expose:
      - "6167"
    environment:
      CONDUIT_SERVER_NAME: {{CONDUIT_SERVER_NAME}}
      CONDUIT_REGISTRATION_TOKEN: {{CONDUIT_REGISTRATION_TOKEN}}
    volumes:
      - data:/var/lib/matrix-conduit

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "CONDUIT_SERVER_NAME",
            label: "Nome do servidor",
            default: None,
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "CONDUIT_REGISTRATION_TOKEN",
            label: "Token de registro",
            default: None,
            required: false,
            secret: true,
        },
    ],
};
