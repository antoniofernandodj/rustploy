use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "netdata",
    name: "Netdata",
    description: "Monitor analítico de infraestrutura em tempo real",
    category: TemplateCategory::Monitoring,
    default_port: 19999,
    compose: r#"
services:
  netdata:
    image: netdata/netdata:latest
    restart: unless-stopped
    ports:
      - "19999"
    volumes:
      - netdataconfig:/etc/netdata
      - netdatalib:/var/lib/netdata
      - netdatacache:/var/cache/netdata

volumes:
  netdataconfig:
  netdatalib:
  netdatacache:
"#,
    variables: &[],
};
