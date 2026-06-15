use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "lobechat",
    name: "Lobe Chat",
    description: "Framework de chat com IA moderno com suporte a plugins de voz",
    category: TemplateCategory::Ai,
    default_port: 3210,
    compose: r#"
services:
  lobechat:
    image: lobehub/lobe-chat:latest
    restart: unless-stopped
    expose:
      - "3210"
    environment:
      OPENAI_API_KEY: {{OPENAI_API_KEY}}
"#,
    variables: &[TemplateVar {
        key: "OPENAI_API_KEY",
        label: "OpenAI API Key",
        default: None,
        required: false,
        secret: true,
    }],
};
