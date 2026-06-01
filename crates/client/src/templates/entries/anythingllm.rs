use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "anythingllm",
    name: "AnythingLLM",
    description: "Chatbot privado para conversar com seus documentos locais",
    category: TemplateCategory::Ai,
    default_port: 3001,
    compose: r#"
services:
  anythingllm:
    image: mintplexlabs/anythingllm:latest
    restart: unless-stopped
    ports:
      - "3001"
    volumes:
      - storage:/app/server/storage

volumes:
  storage:
"#,
    variables: &[],
};
