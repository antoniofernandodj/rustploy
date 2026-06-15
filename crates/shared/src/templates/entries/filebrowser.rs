use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "filebrowser",
    name: "FileBrowser",
    description: "Gerenciador de arquivos web com controle de usuários",
    category: TemplateCategory::DevTools,
    default_port: 80,
    compose: r#"
services:
  filebrowser:
    image: filebrowser/filebrowser:latest
    restart: unless-stopped
    expose:
      - "80"
    volumes:
      - fb_data:/database
      - fb_files:/srv

volumes:
  fb_data:
  fb_files:
"#,
    variables: &[],
};
