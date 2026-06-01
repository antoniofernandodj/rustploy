use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "appwrite",
    name: "Appwrite",
    description: "Backend-as-a-Service (BaaS) completo em Docker",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  appwrite:
    image: appwrite/appwrite:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      _APP_ENV: {{_APP_ENV}}
      _APP_OPENSSL_KEY_V1: {{_APP_OPENSSL_KEY_V1}}
"#,
    variables: &[
        TemplateVar {
            key: "_APP_ENV",
            label: "Ambiente",
            default: Some("production"),
            required: false,
            secret: false,
        },
        TemplateVar {
            key: "_APP_OPENSSL_KEY_V1",
            label: "OpenSSL Key",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
