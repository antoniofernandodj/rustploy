use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "baikal",
    name: "Baikal",
    description: "Servidor CalDAV e CardDAV leve",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  baikal:
    image: ckulka/baikal:nginx
    restart: unless-stopped
    ports:
      - "80"
    volumes:
      - config:/var/www/html/config
      - data:/var/www/html/Specific

volumes:
  config:
  data:
"#,
    variables: &[],
};
