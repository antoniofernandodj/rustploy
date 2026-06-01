use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "soketi",
    name: "Soketi",
    description: "Servidor WebSockets ultrarrápido (Compatível com Pusher/Laravel)",
    category: TemplateCategory::DevTools,
    default_port: 6001,
    compose: r#"
services:
  soketi:
    image: quay.io/soketi/soketi:latest
    restart: unless-stopped
    ports:
      - "6001"
    environment:
      SOKETI_DEFAULT_APP_ID: {{SOKETI_DEFAULT_APP_ID}}
      SOKETI_DEFAULT_APP_KEY: {{SOKETI_DEFAULT_APP_KEY}}
      SOKETI_DEFAULT_APP_SECRET: {{SOKETI_DEFAULT_APP_SECRET}}
"#,
    variables: &[
        TemplateVar {
            key: "SOKETI_DEFAULT_APP_ID",
            label: "App ID",
            default: Some("app-id"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "SOKETI_DEFAULT_APP_KEY",
            label: "App Key",
            default: None,
            required: true,
            secret: true,
        },
        TemplateVar {
            key: "SOKETI_DEFAULT_APP_SECRET",
            label: "App Secret",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
