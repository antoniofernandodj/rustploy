use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "minio",
    name: "MinIO",
    description: "Armazenamento de objetos compatível com S3",
    category: TemplateCategory::Storage,
    default_port: 9000,
    compose: r#"
services:
  minio:
    image: minio/minio:latest
    restart: unless-stopped
    ports:
      - "9000"
      - "9001"
    environment:
      MINIO_ROOT_USER: {{ROOT_USER}}
      MINIO_ROOT_PASSWORD: {{ROOT_PASSWORD}}
    command: server /data --console-address ":9001"
    volumes:
      - minio_data:/data

volumes:
  minio_data:
"#,
    variables: &[
        TemplateVar {
            key: "ROOT_USER",
            label: "Usuário root",
            default: Some("minioadmin"),
            required: true,
            secret: false,
        },
        TemplateVar {
            key: "ROOT_PASSWORD",
            label: "Senha root",
            default: None,
            required: true,
            secret: true,
        },
    ],
};
