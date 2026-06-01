use crate::templates::{Template, TemplateCategory};

pub const TEMPLATE: Template = Template {
    id: "adguard-home",
    name: "AdGuard Home",
    description: "Bloqueador de anúncios e rastreadores em nível de DNS",
    category: TemplateCategory::Networking,
    default_port: 3000,
    compose: r#"
services:
  adguard-home:
    image: adguard/adguardhome:latest
    restart: unless-stopped
    expose:
      - "3000"
    volumes:
      - workdir:/opt/adguardhome/work
      - confdir:/opt/adguardhome/conf

volumes:
  workdir:
  confdir:
"#,
    variables: &[],
};
