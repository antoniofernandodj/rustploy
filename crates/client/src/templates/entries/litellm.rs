use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "litellm",
    name: "LiteLLM",
    description: "Proxy que unifica múltiplos LLMs sob o padrão OpenAI",
    category: TemplateCategory::Ai,
    default_port: 4000,
    compose: r#"
services:
  litellm:
    image: ghcr.io/berriai/litellm:main-latest
    restart: unless-stopped
    expose:
      - "4000"
    environment:
      LITELLM_MASTER_KEY: {{LITELLM_MASTER_KEY}}
"#,
    variables: &[TemplateVar {
        key: "LITELLM_MASTER_KEY",
        label: "Master Key",
        default: None,
        required: true,
        secret: true,
    }],
};
