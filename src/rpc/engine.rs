use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll};

use tokio::sync::oneshot;

use crate::{Consumer, ConsumerError};


struct IdGenerator {
    last_id: usize
}

impl IdGenerator {
    fn new() -> Self {
        Self {
            last_id: 0
        }
    }
    fn next(&mut self) -> usize {
        self.last_id = 0;
        self.last_id
    }
}


pub struct Request<T> {
    result_tx: oneshot::Sender<T>,
    consumer: Consumer,
}

pub struct Response<T> {
    is_broadcast: bool,
    result_rx: oneshot::Receiver<T>,
}

impl<T> Future for Response<T> {
    type Output = Result<T, ConsumerError>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context) -> Poll<Self::Output> {
        if self.is_broadcast {
            Poll::Ready(Err(ConsumerError::Broadcast))
        } else {
            Pin::new(&mut self.result_rx)
                .poll(cx)
                .map_err(|_| ConsumerError::Disconnected)
        }
    }
}

pub struct Engine {
    id_generator: IdGenerator,
    requests: HashMap<usize, oneshot::Sender<Vec<u8>>>,
}

impl Engine {
    pub fn new() -> Self {
        Self {
            id_generator: IdGenerator::new(),
            requests: HashMap::new(),
        }
    }
}

#[tokio::main]
#[test]
async fn test_response_err_broadcast() {
    use tokio::sync::oneshot;
    let (_tx, rx) = oneshot::channel();
    let response: Response<bool> = Response {
        is_broadcast: true,
        result_rx: rx
    };
    assert!(matches!(response.await, Err(ConsumerError::Broadcast)));
}

#[tokio::main]
#[test]
async fn test_response_err_disconnected() {
    use tokio::sync::oneshot;
    let (tx, rx) = oneshot::channel();
    drop(tx);
    let response: Response<bool> = Response {
        is_broadcast: false,
        result_rx: rx
    };
    assert!(matches!(response.await, Err(ConsumerError::Disconnected)));
}
