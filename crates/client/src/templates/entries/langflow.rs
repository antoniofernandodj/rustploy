use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "langflow",
    name: "Langflow",
    description: "Interface low-code para pipelines de RAG e agentes de IA",
    category: TemplateCategory::Ai,
    default_port: 7860,
    compose: r#"
services:
  langflow:
    image: langflowai/langflow:latest
    restart: unless-stopped
    ports:
      - "7860"
    environment:
      LANGFLOW_SECRET_KEY: {{LANGFLOW_SECRET_KEY}}
    volumes:
      - data:/app/langflow

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "LANGFLOW_SECRET_KEY",
        label: "Secret Key",
        default: None,
        required: true,
        secret: true,
    }],
};
