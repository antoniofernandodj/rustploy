use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "cyberchef",
    name: "CyberChef",
    description: "Canivete suíço web para criptografia e análise de dados",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  cyberchef:
    image: mpepping/cyberchef:latest
    restart: unless-stopped
    ports:
      - "8080"
"#,
    variables: &[],
};
