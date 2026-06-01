use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "actual-budget",
    name: "Actual Budget",
    description: "Gerenciador de finanças pessoais rápido e privado",
    category: TemplateCategory::Finance,
    default_port: 5006,
    compose: r#"
services:
  actual-budget:
    image: actualbudget/actual-server:latest
    restart: unless-stopped
    ports:
      - "5006"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
