use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "obsidian-livesync",
    name: "Obsidian LiveSync",
    description: "Servidor CouchDB para sincronização em tempo real das notas do Obsidian",
    category: TemplateCategory::Backup,
    default_port: 5984,
    compose: r#"
services:
  obsidian-livesync:
    image: couchdb:latest
    restart: unless-stopped
    ports:
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
            label: "Usuário CouchDB",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "COUCHDB_PASSWORD",
            label: "Senha CouchDB",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
