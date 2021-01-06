use std::error::Error;
use std::fmt::Display;
use std::sync::Arc;
use std::time::Duration;

use bytes::Bytes;
use futures::future::BoxFuture;
use tokio::sync::Mutex;
use websocket_lite::ClientBuilder;

use crate::{
    rpc::engine::{Engine, EngineListener},
    server::session::Auth,
    Consumer, Provider, ProviderError,
};

type GenericError = Box<dyn Error + 'static + Send + Sync>;
type AsyncClient = websocket_lite::AsyncClient<
    Box<dyn websocket_lite::AsyncNetworkStream + Sync + Send + Unpin + 'static>,
>;

#[derive(Debug)]
pub enum ConnectError {
    AlreadyConnected,
    Transport(GenericError),
}

impl From<GenericError> for ConnectError {
    fn from(e: GenericError) -> Self {
        Self::Transport(e)
    }
}

impl Display for ConnectError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

impl Error for ConnectError {}

type ClientTransport = crate::transport::websocket::WebsocketTransport<
    Box<dyn websocket_lite::AsyncNetworkStream + Sync + Send + Unpin + 'static>,
>;

pub struct Client
where
    Self: Sync + Send,
{
    provider: Arc<dyn Provider<()>>,
    auth: Option<Auth>,
    async_client: Option<AsyncClient>,
    join_handle: Option<tokio::task::JoinHandle<()>>,
    engine: Mutex<Option<Arc<Engine>>>,
}

impl Client {
    pub fn new(provider: Arc<dyn Provider<()>>, auth: Option<Auth>) -> Self {
        Client {
            provider,
            auth,
            async_client: None,
            join_handle: None,
            engine: Mutex::new(None),
        }
    }
    pub async fn connect(self: &Arc<Self>, url: url::Url) -> Result<(), ConnectError> {
        if self.async_client.is_some() {
            return Err(ConnectError::AlreadyConnected);
        }
        let mut builder = ClientBuilder::from_url(url);
        if let Some(auth) = &self.auth {
            builder.add_header(
                hyper::header::AUTHORIZATION.to_string(),
                auth.to_string(),
            );
        }
        // FIXME add auth
        //builder.add_header()
        let client = builder.async_connect().await?;

        /*
        let (mut sink, mut stream): (SplitSink<AsyncClient, Message>, SplitStream<AsyncClient>) =
            client.split();
        self.join_handle = Some(tokio::spawn(async move {
            while let Some(message) = stream.next().await {
                println!("Received message: {:?}", message);
            }
        }));
        */
        let mut engine = Arc::new(Engine::new(ClientTransport::new(client)));
        let engine_listener: Arc<dyn EngineListener + Sync + Send> = self.clone();
        engine.start(Arc::downgrade(&engine_listener));

        *self.engine.lock().await = Some(engine);

        Ok(())
    }
    pub async fn join(self: &Arc<Self>) {
        use std::thread::sleep;
        sleep(Duration::from_secs(100000));
        /* FIXME
        if let Some(join_handle) = &mut self.join_handle {
            drop(join_handle.await);
        }
        */
    }
}

impl EngineListener for Client {
    fn call(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> BoxFuture<Result<Bytes, ProviderError>> {
        // FIXME creating an Arc to () is wasteful. This could be
        // optimized to a static variable instead.
        self.provider.call(&Arc::new(()), service, method, data)
    }
    fn shutdown(&self) {
        /*
        if let Some(server) = self.server.upgrade() {
            server.disconnect(self.id);
        }
        */
    }
}

/*
impl Future for Client {
    type Output = ();

    fn poll(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<Self::Output> {
        match self.join_handle {
            Some(join_handle) => {
                let join_handle = Box::pin(&mut join_handle);
                Future::poll(join_handle, cx)
            },
            None => std::task::Poll::Ready(()),
        }
    }
}
*/

impl Consumer for Client {
    fn notify(&self, service: &str, method: &str, data: Bytes) {
        let guard = self.engine.try_lock();
        guard
            .unwrap()
            .as_ref()
            .unwrap()
            .notify(service, method, data)
    }
    fn request(
        &self,
        service: &str,
        method: &str,
        data: Bytes,
    ) -> crate::service::consumer::Response {
        let guard = self.engine.try_lock();
        guard
            .unwrap()
            .as_ref()
            .unwrap()
            .request(service, method, data)
    }
}
