use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "surrealdb",
    name: "SurrealDB",
    description: "Banco de dados multimodel moderno (relacional, grafos, vetorial)",
    category: TemplateCategory::Database,
    default_port: 8000,
    compose: r#"
services:
  surrealdb:
    image: surrealdb/surrealdb:latest
    restart: unless-stopped
    expose:
      - "8000"
    environment:
      SURREAL_USER: {{SURREAL_USER}}
      SURREAL_PASS: {{SURREAL_PASS}}
    volumes:
      - data:/mydata

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "SURREAL_USER",
            label: "Usuário root",
            default: Some("root"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "SURREAL_PASS",
            label: "Senha root",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
