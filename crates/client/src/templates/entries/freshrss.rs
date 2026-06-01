use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "freshrss",
    name: "FreshRSS",
    description: "Agregador e leitor de feeds RSS rápido e customizável",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  freshrss:
    image: freshrss/freshrss:latest
    restart: unless-stopped
    ports:
      - "80"
    environment:
      ADMIN_PASSWORD: {{ADMIN_PASSWORD}}
    volumes:
      - data:/var/www/FreshRSS/data
      - extensions:/var/www/FreshRSS/extensions

volumes:
  data:
  extensions:
"#,
    variables: &[TemplateVar {
        key: "ADMIN_PASSWORD",
        label: "Senha admin",
        default: None,
        required: true,
        secret: true,
    }],
};
