use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "flowise",
    name: "Flowise",
    description: "Interface no-code para construir cadeias de LLM",
    category: TemplateCategory::Ai,
    default_port: 3000,
    compose: r#"
services:
  flowise:
    image: flowiseai/flowise:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      FLOWISE_USERNAME: {{FLOWISE_USERNAME}}
      FLOWISE_PASSWORD: {{FLOWISE_PASSWORD}}
    volumes:
      - data:/root/.flowise

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "FLOWISE_USERNAME",
            label: "Usuário",
            default: Some("admin"),
            required: false,
            secret: false,
        },
        TemplateVar {
            key: "FLOWISE_PASSWORD",
            label: "Senha",
            default: None,
            required: false,
            secret: true,
        },
    ],
};
