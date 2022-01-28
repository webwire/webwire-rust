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
    pub struct ClientMessage {
        pub text: String,
    }
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
        #[validate(length(min = 0i64, max = 32i64))]
        pub username: String,
        #[validate(length(min = 1i64, max = 2048i64))]
        pub text: String,
    }
    #[derive(Clone, Debug, Eq, PartialEq, :: serde :: Serialize, :: serde :: Deserialize)]
    pub enum SendError {
        PermissionDenied,
    }
    #[::async_trait::async_trait]
    pub trait Client {
        type Error: Into<::webwire::ProviderError>;
        async fn on_message(&self, input: &Message) -> Result<(), Self::Error>;
    }
    pub struct ClientProvider<F>(pub F);
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::NamedProvider<S>
        for ClientProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Client + 'static,
    {
        const NAME: &'static str = "chat.Client";
    }
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::Provider<S> for ClientProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Client + 'static,
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
                    let output = service.on_message(&input).await.map_err(|e| e.into())?;
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
            let data: ::bytes::Bytes = serde_json::to_vec(input)
                .map_err(|e| ::webwire::ConsumerError::SerializerError(e))?
                .into();
            let output = self.0.request("chat.Client", "on_message", data).await?;
            let response = ::serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
    #[::async_trait::async_trait]
    pub trait Server {
        type Error: Into<::webwire::ProviderError>;
        async fn send(
            &self,
            input: &ClientMessage,
        ) -> Result<std::result::Result<(), SendError>, Self::Error>;
    }
    pub struct ServerProvider<F>(pub F);
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::NamedProvider<S>
        for ServerProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Server + 'static,
    {
        const NAME: &'static str = "chat.Server";
    }
    impl<F: Sync + Send, S: Sync + Send, T: Sync + Send> ::webwire::Provider<S> for ServerProvider<F>
    where
        F: Fn(::std::sync::Arc<S>) -> T,
        T: Server + 'static,
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
                    let input = serde_json::from_slice::<ClientMessage>(&input)
                        .map_err(::webwire::ProviderError::DeserializerError)?;
                    ::validator::Validate::validate(&input)
                        .map_err(::webwire::ProviderError::ValidationError)?;
                    let output = service.send(&input).await.map_err(|e| e.into())?;
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
            input: &ClientMessage,
        ) -> Result<std::result::Result<(), SendError>, ::webwire::ConsumerError> {
            let data: ::bytes::Bytes = serde_json::to_vec(input)
                .map_err(|e| ::webwire::ConsumerError::SerializerError(e))?
                .into();
            let output = self.0.request("chat.Server", "send", data).await?;
            let response = ::serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
}
