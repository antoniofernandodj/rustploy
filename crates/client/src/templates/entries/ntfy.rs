use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "ntfy",
    name: "NTFY",
    description: "Notificações push para celulares via requisições HTTP simples",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  ntfy:
    image: binwiederhier/ntfy:latest
    restart: unless-stopped
    expose:
      - "80"
    volumes:
      - cache:/var/cache/ntfy
      - etc:/etc/ntfy

volumes:
  cache:
  etc:
"#,
    variables: &[],
};
