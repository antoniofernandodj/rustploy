use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "vaultwarden",
    name: "Vaultwarden",
    description: "Gerenciador de senhas compatível com Bitwarden",
    category: TemplateCategory::Security,
    default_port: 80,
    compose: r#"
services:
  vaultwarden:
    image: vaultwarden/server:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      ADMIN_TOKEN: {{ADMIN_TOKEN}}
    volumes:
      - vw_data:/data

volumes:
  vw_data:
"#,
    variables: &[TemplateVar {
        key: "ADMIN_TOKEN",
        label: "Token admin (argon2 hash recomendado)",
        default: None,
        required: true,
        secret: true,
    }],
};
