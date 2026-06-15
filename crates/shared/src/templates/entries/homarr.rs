use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "homarr",
    name: "Homarr",
    description: "Dashboard moderno de aplicativos residenciais integrado ao Docker",
    category: TemplateCategory::DevTools,
    default_port: 7575,
    compose: r#"
services:
  homarr:
    image: ghcr.io/ajnart/homarr:latest
    restart: unless-stopped
    expose:
      - "7575"
    volumes:
      - configs:/app/data/configs
      - icons:/app/public/icons
      - data:/data

volumes:
  configs:
  icons:
  data:
"#,
    variables: &[],
};
