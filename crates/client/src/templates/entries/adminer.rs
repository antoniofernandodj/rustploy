use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "adminer",
    name: "Adminer",
    description: "Gerenciador de banco de dados leve (MySQL, Postgres, SQLite)",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  adminer:
    image: adminer:latest
    restart: unless-stopped
    expose:
      - "8080"
"#,
    variables: &[],
};
