use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "linkding",
    name: "Linkding",
    description: "Gerenciador de favoritos veloz e minimalista",
    category: TemplateCategory::DevTools,
    default_port: 9090,
    compose: r#"
services:
  linkding:
    image: sissbruecker/linkding:latest
    restart: unless-stopped
    expose:
      - "9090"
    environment:
      LD_SUPERUSER_NAME: {{ADMIN_USER}}
      LD_SUPERUSER_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - linkding_data:/etc/linkding/data

volumes:
  linkding_data:
"#,
    variables: &[
        TemplateVar {
            key: "ADMIN_USER",
            label: "Usuário admin",
            default: Some("admin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "ADMIN_PASSWORD",
            label: "Senha admin",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
