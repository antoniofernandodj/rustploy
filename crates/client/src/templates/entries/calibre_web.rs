use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "calibre-web",
    name: "Calibre-Web",
    description: "Interface para e-books do Calibre via navegador",
    category: TemplateCategory::Media,
    default_port: 8083,
    compose: r#"
services:
  calibre-web:
    image: lscr.io/linuxserver/calibre-web:latest
    restart: unless-stopped
    expose:
      - "8083"
    volumes:
      - config:/config
      - books:/books

volumes:
  config:
  books:
"#,
    variables: &[],
};
