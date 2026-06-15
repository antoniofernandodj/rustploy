use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "rustdesk",
    name: "RustDesk",
    description: "Servidor de acesso remoto (Alternativa ao TeamViewer/Anydesk)",
    category: TemplateCategory::Networking,
    default_port: 21115,
    compose: r#"
services:
  rustdesk:
    image: rustdesk/rustdesk-server:latest
    restart: unless-stopped
    expose:
      - "21115"
    volumes:
      - data:/root

volumes:
  data:
"#,
    variables: &[],
};
