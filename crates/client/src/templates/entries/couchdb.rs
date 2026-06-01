use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "couchdb",
    name: "CouchDB",
    description: "Banco de dados NoSQL baseado em documentos com boa sincronização",
    category: TemplateCategory::Database,
    default_port: 5984,
    compose: r#"
services:
  couchdb:
    image: couchdb:latest
    restart: unless-stopped
    expose:
      - "5984"
    environment:
      COUCHDB_USER: {{COUCHDB_USER}}
      COUCHDB_PASSWORD: {{COUCHDB_PASSWORD}}
    volumes:
      - data:/opt/couchdb/data

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "COUCHDB_USER",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "COUCHDB_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
