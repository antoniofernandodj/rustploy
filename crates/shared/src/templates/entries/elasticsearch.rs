use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "elasticsearch",
    name: "Elasticsearch",
    description: "Motor distribuído de busca textual e análise analítica",
    category: TemplateCategory::Database,
    default_port: 9200,
    compose: r#"
services:
  elasticsearch:
    image: elasticsearch:8.11.1
    restart: unless-stopped
    expose:
      - "9200"
    environment:
      ELASTIC_PASSWORD: {{ELASTIC_PASSWORD}}
      discovery.type: {{discovery.type}}
    volumes:
      - data:/usr/share/elasticsearch/data

volumes:
  data:
"#,
    variables: &[
        TemplateVar {
            key: "ELASTIC_PASSWORD",
            label: "Senha Elasticsearch",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "discovery.type",
            label: "Modo single-node",
            default: Some("single-node"),
            required: false,
            secret: false,
        },
    ],
};
