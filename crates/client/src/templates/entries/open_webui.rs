use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "open-webui",
    name: "Open WebUI",
    description: "Interface web para modelos LLM locais (Ollama) estilo ChatGPT",
    category: TemplateCategory::Ai,
    default_port: 3000,
    compose: r#"
services:
  open-webui:
    image: ghcr.io/open-webui/open-webui:main
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      WEBUI_SECRET_KEY: {{WEBUI_SECRET_KEY}}
    volumes:
      - data:/app/backend/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "WEBUI_SECRET_KEY",
        label: "Secret Key",
        default: None,
        required: true,
        secret: true,
    }],
};
