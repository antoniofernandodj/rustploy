use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "azuracast",
    name: "AzuraCast",
    description: "Painel completo para gerenciamento de Web Rádios",
    category: TemplateCategory::Media,
    default_port: 80,
    compose: r#"
services:
  azuracast:
    image: ghcr.io/azuracast/azuracast:latest
    restart: unless-stopped
    expose:
      - "80"
    volumes:
      - station_data:/var/azuracast/stations

volumes:
  station_data:
"#,
    variables: &[],
};
