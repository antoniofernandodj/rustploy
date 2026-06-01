use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "clickhouse",
    name: "ClickHouse",
    description: "Banco de dados analítico (OLAP) orientado a colunas extremamente veloz",
    category: TemplateCategory::Database,
    default_port: 8123,
    compose: r#"
services:
  clickhouse:
    image: clickhouse/clickhouse-server:latest
    restart: unless-stopped
    expose:
      - "8123"
    volumes:
      - data:/var/lib/clickhouse

volumes:
  data:
"#,
    variables: &[],
};
