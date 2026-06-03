use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "linkstack",
    name: "LinkStack",
    description: "Plataforma estilo Link na Bio altamente customizável",
    category: TemplateCategory::Cms,
    default_port: 80,
    compose: r#"
services:
  linkstack:
    image: linkstackorg/linkstack:latest
    restart: unless-stopped
    expose:
      - "80"
    environment:
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - data:/htdocs

volumes:
  data:
"#,
    variables: &[TemplateVar {
        key: "ADMIN_PASSWORD",
        label: "Senha admin",
        default: None,
        required: true,
        secret: true,
    }],
};
