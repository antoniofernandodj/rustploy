use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "dashy",
    name: "Dashy",
    description: "Dashboard pessoal customizável com monitoramento de status",
    category: TemplateCategory::DevTools,
    default_port: 8080,
    compose: r#"
services:
  dashy:
    image: lissy93/dashy:latest
    restart: unless-stopped
    expose:
      - "8080"
    volumes:
      - conf.yml:/app/public/conf.yml

volumes:
  conf.yml:
"#,
    variables: &[],
};
