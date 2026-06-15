use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "garage-s3",
    name: "Garage S3",
    description: "Armazenamento de objetos distribuído compatível com S3",
    category: TemplateCategory::Storage,
    default_port: 3900,
    compose: r#"
services:
  garage-s3:
    image: dxflrs/garage:latest
    restart: unless-stopped
    expose:
      - "3900"
    volumes:
      - data:/var/lib/garage/data
      - meta:/var/lib/garage/meta

volumes:
  data:
  meta:
"#,
    variables: &[],
};
