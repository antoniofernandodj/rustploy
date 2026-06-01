use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "arangodb",
    name: "ArangoDB",
    description: "Banco de dados multi-modelo (grafos, documentos, KV)",
    category: TemplateCategory::Database,
    default_port: 8529,
    compose: r#"
services:
  arangodb:
    image: arangodb:latest
    restart: unless-stopped
    expose:
      - "8529"
    environment:
      ARANGO_ROOT_PASSWORD: {{ARANGO_ROOT_PASSWORD}}
    volumes:
      - data:/var/lib/arangodb3

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "ARANGO_ROOT_PASSWORD",
        label: "Senha root",
        default: None,
        required: true,
        secret: true,
    }],
};
