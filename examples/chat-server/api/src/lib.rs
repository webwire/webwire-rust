#[allow(dead_code)]
pub mod chat {
    #[derive(Clone, Debug, Eq, PartialEq, :: serde :: Serialize, :: serde :: Deserialize)]
    pub struct Message {
        pub text: String,
    }
    #[derive(Clone, Debug, Eq, PartialEq, :: serde :: Serialize, :: serde :: Deserialize)]
    pub enum SendError {
        PermissionDenied,
    }
    #[::async_trait::async_trait]
    pub trait Server<S: ::std::marker::Sync + ::std::marker::Send> {
        async fn send(
            &self,
            request: &::webwire::Request<S>,
            input: &Message,
        ) -> Result<Result<(), SendError>, ::webwire::ProviderError>;
    }
    pub struct ServerProvider<S, T>(pub T, ::std::marker::PhantomData<S>)
    where
        S: ::std::marker::Sync + ::std::marker::Send,
        T: Server<S> + ::std::marker::Sync + ::std::marker::Send;
    impl<
            S: ::std::marker::Sync + ::std::marker::Send,
            T: Server<S> + ::std::marker::Sync + ::std::marker::Send,
        > ServerProvider<S, T>
    {
        pub fn new(service: T) -> Self {
            Self(service, ::std::marker::PhantomData::<S>::default())
        }
    }
    #[::async_trait::async_trait]
    impl<
            S: ::std::marker::Sync + ::std::marker::Send,
            T: Server<S> + ::std::marker::Sync + ::std::marker::Send,
        > ::webwire::Provider<S> for ServerProvider<S, T>
    {
        fn name(&self) -> &'static str {
            "Server"
        }
        async fn call(
            &self,
            request: &::webwire::Request<S>,
            data: ::bytes::Bytes,
        ) -> ::webwire::Response<::bytes::Bytes> {
            match &*request.method {
                "send" => {
                    let input = serde_json::from_slice(&data)
                        .map_err(|e| ::webwire::ProviderError::DeserializerError(e))?;
                    let response = (self.0).send(request, &input).await?;
                    let output = serde_json::to_vec(&response)
                        .map_err(|e| ::webwire::ProviderError::SerializerError(e))?;
                    Ok(output.into())
                }
                _ => Err(::webwire::ProviderError::MethodNotFound),
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
            let output = self.0.call("Server", "send", data.into()).await?;
            let response = serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
    #[::async_trait::async_trait]
    pub trait Client<S: ::std::marker::Sync + ::std::marker::Send> {
        async fn on_message(
            &self,
            request: &::webwire::Request<S>,
            input: &Message,
        ) -> Result<(), ::webwire::ProviderError>;
    }
    pub struct ClientProvider<S, T>(pub T, ::std::marker::PhantomData<S>)
    where
        S: ::std::marker::Sync + ::std::marker::Send,
        T: Client<S> + ::std::marker::Sync + ::std::marker::Send;
    impl<
            S: ::std::marker::Sync + ::std::marker::Send,
            T: Client<S> + ::std::marker::Sync + ::std::marker::Send,
        > ClientProvider<S, T>
    {
        pub fn new(service: T) -> Self {
            Self(service, ::std::marker::PhantomData::<S>::default())
        }
    }
    #[::async_trait::async_trait]
    impl<
            S: ::std::marker::Sync + ::std::marker::Send,
            T: Client<S> + ::std::marker::Sync + ::std::marker::Send,
        > ::webwire::Provider<S> for ClientProvider<S, T>
    {
        fn name(&self) -> &'static str {
            "Client"
        }
        async fn call(
            &self,
            request: &::webwire::Request<S>,
            data: ::bytes::Bytes,
        ) -> ::webwire::Response<::bytes::Bytes> {
            match &*request.method {
                "on_message" => {
                    let input = serde_json::from_slice(&data)
                        .map_err(|e| ::webwire::ProviderError::DeserializerError(e))?;
                    let response = (self.0).on_message(request, &input).await?;
                    let output = serde_json::to_vec(&response)
                        .map_err(|e| ::webwire::ProviderError::SerializerError(e))?;
                    Ok(output.into())
                }
                _ => Err(::webwire::ProviderError::MethodNotFound),
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
            let output = self.0.call("Client", "on_message", data.into()).await?;
            let response = serde_json::from_slice(&output)
                .map_err(|e| ::webwire::ConsumerError::DeserializerError(e))?;
            Ok(response)
        }
    }
}
