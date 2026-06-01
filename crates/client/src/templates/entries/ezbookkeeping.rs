use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "ezbookkeeping",
    name: "EZBookkeeping",
    description: "Gerenciador contábil para finanças pessoais",
    category: TemplateCategory::Finance,
    default_port: 8080,
    compose: r#"
services:
  ezbookkeeping:
    image: mayswind/ezbookkeeping:latest
    restart: unless-stopped
    ports:
      - "8080"
    environment:
      EZB_SECRET_KEY: {{EZB_SECRET_KEY}}
    volumes:
      - data:/ezbookkeeping/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "EZB_SECRET_KEY",
        label: "Secret Key",
        default: None,
        required: true,
        secret: true,
    }],
};
