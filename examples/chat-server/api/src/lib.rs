#[allow(dead_code)]
pub mod chat {
    #[derive(
        Clone,
        Debug,
        Eq,
        PartialEq,
        :: serde :: Serialize,
        :: serde :: Deserialize,
        :: validator :: Validate,
    )]
    pub struct Message {
        #[validate(length(min = 1i64, max = 2048i64))]
        pub text: String,
    }
    #[derive(Clone, Debug, Eq, PartialEq, :: serde :: Serialize, :: serde :: Deserialize)]
    pub enum SendError {
        PermissionDenied,
    }
    #[::async_trait::async_trait]
    pub trait Client<S: ::std::marker::Sync + ::std::marker::Send> {
        async fn on_message(&self, input: &Message) -> Result<(), ::webwire::ProviderError>;
    }
    pub struct ClientProvider<F>(pub F);
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::NamedProvider<S>
        for ClientProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Client<S> + 'static,
    {
        const NAME: &'static str = "Client";
    }
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::Provider<S> for ClientProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Client<S> + 'static,
    {
        fn call(
            &self,
            session: &::std::sync::Arc<S>,
            _service: &str,
            method: &str,
            input: ::bytes::Bytes,
        ) -> ::futures::future::BoxFuture<'static, Result<::bytes::Bytes, ::webwire::ProviderError>>
        {
            let service = self.0(session.clone());
            match method {
                "on_message" => Box::pin(async move {
                    let input = serde_json::from_slice::<Message>(&input)
                        .map_err(::webwire::ProviderError::DeserializerError)?;
                    ::validator::Validate::validate(&input)
                        .map_err(::webwire::ProviderError::ValidationError)?;
                    let output = service.on_message(&input).await?;
                    let response = serde_json::to_vec(&output)
                        .map_err(|e| ::webwire::ProviderError::SerializerError(e))
                        .map(::bytes::Bytes::from)?;
                    Ok(response)
                }),
                _ => Box::pin(::futures::future::ready(Err(
                    ::webwire::ProviderError::MethodNotFound,
                ))),
            }
        }
    }
    pub struct ClientConsumer<'a>(
        pub &'a (dyn ::webwire::Consumer + ::std::marker::Sync + ::std::marker::Send),
    );
    impl<'a> ClientConsumer<'a> {
        pub async fn on_message(&self, input: &Message) -> Result<(), ::webwire::ConsumerError> {
            let data = serde_json::to_vec(input)
                .map_err(|e| ::webwire::ConsumerError::SerializerError(e))?;
            let output = self.0.request("Client", "on_message", data.into()).await?;
            let response = serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
    #[::async_trait::async_trait]
    pub trait Server<S: ::std::marker::Sync + ::std::marker::Send> {
        async fn send(
            &self,
            input: &Message,
        ) -> Result<Result<(), SendError>, ::webwire::ProviderError>;
    }
    pub struct ServerProvider<F>(pub F);
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::NamedProvider<S>
        for ServerProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Server<S> + 'static,
    {
        const NAME: &'static str = "Server";
    }
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::Provider<S> for ServerProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Server<S> + 'static,
    {
        fn call(
            &self,
            session: &::std::sync::Arc<S>,
            _service: &str,
            method: &str,
            input: ::bytes::Bytes,
        ) -> ::futures::future::BoxFuture<'static, Result<::bytes::Bytes, ::webwire::ProviderError>>
        {
            let service = self.0(session.clone());
            match method {
                "send" => Box::pin(async move {
                    let input = serde_json::from_slice::<Message>(&input)
                        .map_err(::webwire::ProviderError::DeserializerError)?;
                    ::validator::Validate::validate(&input)
                        .map_err(::webwire::ProviderError::ValidationError)?;
                    let output = service.send(&input).await?;
                    let response = serde_json::to_vec(&output)
                        .map_err(|e| ::webwire::ProviderError::SerializerError(e))
                        .map(::bytes::Bytes::from)?;
                    Ok(response)
                }),
                _ => Box::pin(::futures::future::ready(Err(
                    ::webwire::ProviderError::MethodNotFound,
                ))),
            }
        }
    }
    pub struct ServerConsumer<'a>(
        pub &'a (dyn ::webwire::Consumer + ::std::marker::Sync + ::std::marker::Send),
    );
    impl<'a> ServerConsumer<'a> {
        pub async fn send(
            &self,
            input: &Message,
        ) -> Result<Result<(), SendError>, ::webwire::ConsumerError> {
            let data = serde_json::to_vec(input)
                .map_err(|e| ::webwire::ConsumerError::SerializerError(e))?;
            let output = self.0.request("Server", "send", data.into()).await?;
            let response = serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
}
