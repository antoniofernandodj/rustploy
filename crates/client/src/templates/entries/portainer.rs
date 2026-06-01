use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "portainer",
    name: "Portainer",
    description: "Painel visual para gerenciamento de containers Docker",
    category: TemplateCategory::DevTools,
    default_port: 9000,
    compose: r#"
services:
  portainer:
    image: portainer/portainer-ce:latest
    restart: unless-stopped
    expose:
      - "9000"
      - "9443"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - portainer_data:/data

volumes:
  portainer_data:
"#,
    variables: &[],
};
