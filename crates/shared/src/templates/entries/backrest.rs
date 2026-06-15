use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "backrest",
    name: "Backrest",
    description: "Interface web para backups automatizados via restic",
    category: TemplateCategory::Backup,
    default_port: 9898,
    compose: r#"
services:
  backrest:
    image: garethgeorge/backrest:latest
    restart: unless-stopped
    expose:
      - "9898"
    volumes:
      - data:/data
      - config:/etc/backrest
      - cache:/var/cache/backrest

volumes:
  data:
  config:
  cache:
"#,
    variables: &[],
};
