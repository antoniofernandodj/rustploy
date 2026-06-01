use crate::templates::{Template, TemplateCategory, TemplateVar};

pub const TEMPLATE: Template = Template {
    id: "rocketchat",
    name: "Rocket.Chat",
    description: "Ecossistema completo de chat e comunicação corporativa",
    category: TemplateCategory::Communication,
    default_port: 3000,
    compose: r#"
services:
  mongo:
    image: mongo:6
    restart: unless-stopped
    command: mongod --oplogSize 128 --replSet rs0
    volumes:
      - mongo_data:/data/db

  mongo-init-replica:
    image: mongo:6
    command: >
      bash -c "sleep 5 && mongosh --host mongo:27017 --eval \"rs.initiate({_id:'rs0',members:[{_id:0,host:'mongo:27017'}]})\""
    depends_on:
      - mongo

  rocketchat:
    image: registry.rocket.chat/rocketchat/rocket.chat:latest
    restart: unless-stopped
    ports:
      - "3000"
    environment:
      MONGO_URL: mongodb://mongo:27017/rocketchat?replicaSet=rs0
      MONGO_OPLOG_URL: mongodb://mongo:27017/local?replicaSet=rs0
      ROOT_URL: http://{{DOMAIN}}
      PORT: 3000
    depends_on:
      - mongo

volumes:
  mongo_data:
"#,
    variables: &[TemplateVar {
        key: "DOMAIN",
        label: "URL do site",
        default: Some("localhost:3000"),
        required: true,
        secret: false,
    }],
};
