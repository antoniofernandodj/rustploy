use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "mailpit",
    name: "Mailpit",
    description: "Servidor SMTP falso para testes e inspeção de e-mails",
    category: TemplateCategory::DevTools,
    default_port: 8025,
    compose: r#"
services:
  mailpit:
    image: axllent/mailpit:latest
    restart: unless-stopped
    expose:
      - "8025"
    volumes:
      - data:/data

volumes:
  data:
"#,
    variables: &[],
};
