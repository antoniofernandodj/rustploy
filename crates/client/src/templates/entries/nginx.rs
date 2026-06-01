use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "nginx",
    name: "Nginx",
    description: "Servidor web de alta performance e proxy reverso",
    category: TemplateCategory::Networking,
    default_port: 80,
    compose: r#"
services:
  nginx:
    image: nginx:latest
    restart: unless-stopped
    expose:
      - "80"
    volumes:
      - html:/usr/share/nginx/html

volumes:
  html:
"#,
    variables: &[],
};
