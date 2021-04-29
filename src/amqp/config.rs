use lapin::Connection;
use serde::Deserialize;
use tokio_amqp::LapinTokioExt;

#[derive(Clone, Debug, Deserialize)]
pub enum AMQPScheme {
    AMQP,
    AMQPS,
}

impl Default for AMQPScheme {
    fn default() -> Self {
        Self::AMQP
    }
}

fn default_vhost() -> String {
    String::from("/")
}

#[derive(Clone, Debug, Deserialize)]
pub struct AMQPConfig {
    pub scheme: AMQPScheme,
    pub host: String,
    pub port: Option<u16>,
    #[serde(default = "default_vhost")]
    pub vhost: String,
    pub username: String,
    pub password: String,
    pub exchange_name: String,
    pub queue_name: String,
}

impl Default for AMQPConfig {
    fn default() -> Self {
        Self {
            scheme: AMQPScheme::default(),
            host: "127.0.0.1".to_owned(),
            port: None,
            vhost: "/".to_owned(),
            username: "".to_owned(),
            password: "".to_owned(),
            // FIXME having defaults for this might not be
            // a good idea after all. The code should bail out
            // with an error if this is missing!
            exchange_name: "".to_owned(),
            queue_name: "".to_owned(),
        }
    }
}

impl AMQPConfig {
    pub async fn connect(&self) -> Result<Connection, lapin::Error> {
        let connection_properties = lapin::ConnectionProperties::default().with_tokio();
        let scheme = match &self.scheme {
            AMQP => lapin::uri::AMQPScheme::AMQP,
            AMQPS => lapin::uri::AMQPScheme::AMQPS,
        };
        let port = self.port.unwrap_or_else(|| scheme.default_port());
        let uri = lapin::uri::AMQPUri {
            scheme,
            authority: lapin::uri::AMQPAuthority {
                userinfo: lapin::uri::AMQPUserInfo {
                    username: self.username.clone(),
                    password: self.password.clone(),
                },
                host: self.host.clone(),
                port,
            },
            vhost: self.vhost.clone(),
            query: lapin::uri::AMQPQueryString {
                // FIXME add support for this?
                ..Default::default()
            },
        };
        Connection::connect_uri(uri, connection_properties).await
    }
}
