use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "syncthing",
    name: "Syncthing",
    description: "Sincronizador contínuo e descentralizado de diretórios",
    category: TemplateCategory::Backup,
    default_port: 8384,
    compose: r#"
services:
  syncthing:
    image: syncthing/syncthing:latest
    restart: unless-stopped
    expose:
      - "8384"
    volumes:
      - data:/var/syncthing

volumes:
  data:
"#,
    variables: &[],
};
