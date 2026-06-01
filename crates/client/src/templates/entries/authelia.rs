use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "authelia",
    name: "Authelia",
    description: "Provedor de SSO com autenticação multifator (2FA)",
    category: TemplateCategory::Security,
    default_port: 9091,
    compose: r#"
services:
  authelia:
    image: authelia/authelia:latest
    restart: unless-stopped
    ports:
      - "9091"
    volumes:
      - config:/config

volumes:
  config:
"#,
    variables: &[],
};
