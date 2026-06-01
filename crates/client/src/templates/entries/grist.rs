use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "grist",
    name: "Grist",
    description: "Planilha inteligente integrada com banco de dados relacional",
    category: TemplateCategory::DevTools,
    default_port: 8484,
    compose: r#"
services:
  grist:
    image: gristlabs/grist:latest
    restart: unless-stopped
    ports:
      - "8484"
    environment:
      GRIST_SESSION_SECRET: {{GRIST_SESSION_SECRET}}
    volumes:
      - data:/persist

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "GRIST_SESSION_SECRET",
        label: "Session Secret",
        default: None,
        required: true,
        secret: true,
    }],
};
