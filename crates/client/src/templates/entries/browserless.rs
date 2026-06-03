use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "browserless",
    name: "Browserless",
    description: "Execução remota e headless do Chrome/Puppeteer em containers",
    category: TemplateCategory::DevTools,
    default_port: 3000,
    compose: r#"
services:
  browserless:
    image: browserless/chrome:latest
    restart: unless-stopped
    expose:
      - "3000"
    environment:
      TOKEN: {{TOKEN}}
"#,
    variables: &[TemplateVar {
        key: "TOKEN",
        label: "API Token",
        default: None,
        required: true,
        secret: true,
    }],
};
