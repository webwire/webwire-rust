//! Hyper support

use std::convert::Infallible;
use std::sync::Arc;
use std::task::{Context, Poll};

use futures::future::BoxFuture;
use hyper::server::conn::AddrStream;
use hyper::{header, http, upgrade::Upgraded, Body, Request, Response, StatusCode};
use hyper_websocket_lite::{server_upgrade, AsyncClient};

use super::session::{Auth, AuthError};

type WebsocketTransport = crate::transport::websocket::WebsocketTransport<Upgraded>;

async fn on_client<S: Sync + Send + 'static>(
    client: AsyncClient,
    server: Arc<super::Server<S>>,
    session: S,
) {
    let transport = WebsocketTransport::new(client);
    server.connect(transport, session).await;
}

async fn upgrade<S>(
    request: Request<Body>,
    server: Arc<super::Server<S>>,
) -> Result<Response<Body>, Infallible>
where
    S: Sync + Send + 'static,
{
    let auth = match request.headers().get(hyper::header::AUTHORIZATION) {
        Some(value) => match Auth::parse(value.as_bytes()) {
            Some(auth) => Some(auth),
            None => {
                return Ok(Response::builder()
                    .status(StatusCode::UNAUTHORIZED)
                    .header(header::CONTENT_TYPE, "text/plain")
                    .body(Body::from("Unauthorized: Invalid authentication header"))
                    .unwrap());
            }
        },
        None => None,
    };
    let session = match server.auth(auth).await {
        Ok(session) => session,
        Err(AuthError::Unauthorized) => {
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Unauthorized: Invalid credentials"))
                .unwrap());
        }
        Err(AuthError::Invalid) => {
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Unauthorized: Invalid authentication header"))
                .unwrap());
        }
        Err(AuthError::Unsupported) => {
            return Ok(Response::builder()
                .status(StatusCode::UNAUTHORIZED)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from(
                    "Unauthorized: Unsupported authentication method",
                ))
                .unwrap());
        }
        Err(AuthError::InternalServerError) => {
            return Ok(Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Internal Server Error"))
                .unwrap());
        }
    };
    let on_client_fut = |client| on_client(client, server, session);
    Ok(match server_upgrade(request, on_client_fut).await {
        Ok(response) => response,
        Err(e) => {
            tracing::error!("Internal server error: {:?}", e);
            Response::builder()
                .status(StatusCode::INTERNAL_SERVER_ERROR)
                .header(header::CONTENT_TYPE, "text/plain")
                .body(Body::from("Internal Server Error"))
                .unwrap()
        }
    })
}

/// The hyper service returned by `MakeService`.
pub struct HyperService<S: Sync + Send> {
    /// A ready configured webwire Server object used to handle
    /// incoming connections and requests.
    pub server: Arc<super::Server<S>>,
}

impl<S: Sync + Send + 'static> hyper::service::Service<Request<Body>> for HyperService<S> {
    type Response = Response<Body>;
    type Error = Infallible;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, request: Request<Body>) -> Self::Future {
        Box::pin(upgrade(request, self.server.clone()))
    }
}

/// The `MakeService` used to construct hyper services.
pub struct MakeHyperService<S: Sync + Send> {
    /// A ready configured webwire Server object used to handle
    /// incoming connections and requests.
    pub server: Arc<super::Server<S>>,
}

impl<S: Sync + Send + 'static> hyper::service::Service<&AddrStream> for MakeHyperService<S> {
    type Response = HyperService<S>;
    type Error = http::Error;
    type Future = BoxFuture<'static, Result<Self::Response, Self::Error>>;

    fn poll_ready(&mut self, _cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, _conn: &AddrStream) -> Self::Future {
        let server = self.server.clone();
        let fut = async move { Ok(HyperService { server }) };
        Box::pin(fut)
    }
}
