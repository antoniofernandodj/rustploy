use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "vault",
    name: "Vault",
    description: "Cofre HashiCorp para gerenciamento estrito de segredos",
    category: TemplateCategory::Security,
    default_port: 8200,
    compose: r#"
services:
  vault:
    image: hashicorp/vault:latest
    restart: unless-stopped
    ports:
      - "8200"
    environment:
      VAULT_DEV_ROOT_TOKEN_ID: {{VAULT_DEV_ROOT_TOKEN_ID}}
    volumes:
      - data:/vault/data

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "VAULT_DEV_ROOT_TOKEN_ID",
        label: "Root Token (modo dev)",
        default: None,
        required: true,
        secret: true,
    }],
};
