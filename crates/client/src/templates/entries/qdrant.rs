use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "qdrant",
    name: "Qdrant",
    description: "Banco de dados vetorial para busca de similaridade e embeddings",
    category: TemplateCategory::Database,
    default_port: 6333,
    compose: r#"
services:
  qdrant:
    image: qdrant/qdrant:latest
    restart: unless-stopped
    ports:
      - "6333"
    volumes:
      - data:/qdrant/storage

volumes:
  data:
"#,
    variables: &[],
};
