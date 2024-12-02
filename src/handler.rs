#![allow(non_snake_case)]
use std::{future::Future, marker::PhantomData, pin::Pin};

use crate::{
    extract::FromMesasge,
    request::Request,
    response::{IntoResponse, Response},
};

pub trait Handler<T, S>: Clone + Send + Sync + 'static {
    type Future: Future<Output = Response> + Send + 'static;
    fn call(self, args: Request, state: S) -> Self::Future;
}

impl<F, Fut, S, Res> Handler<(), S> for F
where
    F: Fn() -> Fut + Clone + Send + Sync + 'static,
    Fut: Future<Output = Res> + Send + 'static,
    Res: IntoResponse,
    S: Clone + Send + Sync + 'static,
{
    type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

    fn call(self, _: Request, _: S) -> Self::Future {
        let fut = self();
        Box::pin(async move { fut.await.into_response() })
    }
}

macro_rules! impl_handler {
    ($($ty:ident),*) => {
        impl<F, Fut, $($ty,)* S, Res> Handler<($($ty,)*), S> for F
        where
            F: Fn($($ty,)*) -> Fut + Clone + Send + Sync + 'static,
            Fut: Future<Output = Res> + Send + 'static,
         $( $ty: FromMesasge<S> + Send + Sync + 'static, )*
            Res: IntoResponse,
            S: Clone + Send + Sync + 'static,
        {
            type Future = Pin<Box<dyn Future<Output = Response> + Send>>;

            fn call(self, req: Request, state: S) -> Self::Future {
             $( let $ty = match $ty::call(&req, state.clone()) {
                              Ok(e) => e,
                              Err(e) => return Box::pin(async move { e.into_response() }),
                          };
             )*
                let fut = self($($ty,)*);
                Box::pin(async move { fut.await.into_response() })
            }
        }
    };
}

impl_handler!(T1);
impl_handler!(T1, T2);

// Type Erasure
pub trait HandlerService<S> {
    fn call(&self, req: Request, state: S) -> Pin<Box<dyn Future<Output = Response> + Send>>;
}

pub struct ExtractorHandler<H, T, S>
where
    H: Handler<T, S>,
{
    pub handler: H,
    pub _marker: PhantomData<(T, S)>,
}

impl<H, T, S> HandlerService<S> for ExtractorHandler<H, T, S>
where
    H: Handler<T, S> + Clone + Send + Sync + 'static,
    T: Clone + Send + Sync + 'static,
{
    fn call(
        &self,
        req: Request,
        state: S,
    ) -> std::pin::Pin<Box<dyn std::future::Future<Output = Response> + Send>> {
        let fut = self.handler.clone().call(req, state);
        Box::pin(fut)
    }
}

#[cfg(test)]
mod tests {
    use std::{marker::PhantomData, sync::Arc};

    use bytes::Bytes;

    use crate::{
        error::ExtractError,
        extract::FromMesasge,
        handler::{ExtractorHandler, HandlerService},
        request::{Frames, Request},
        response::{IntoResponse, Response},
    };

    #[derive(Clone)]
    struct MockExtractor(String);

    impl<S> FromMesasge<S> for MockExtractor {
        type Rejection = ExtractError;

        fn call(req: &Request, _: S) -> Result<Self, Self::Rejection> {
            Ok(MockExtractor(req.try_to_string()?))
        }
    }

    #[derive(Debug, PartialEq)]
    struct MockResponse(String);

    impl IntoResponse for MockResponse {
        fn into_response(self) -> Response {
            Response::ok(self.0)
        }
    }

    #[tokio::test]
    async fn test_single_extractor_handler() {
        async fn handler(arg: MockExtractor) -> MockResponse {
            MockResponse(format!("Processed: {}", arg.0))
        }

        let request = Request::new(Frames::Text, Bytes::from("test_data"));

        let handler_service = ExtractorHandler {
            handler,
            _marker: PhantomData,
        };

        let response = handler_service.call(request, ()).await;
        assert_eq!(response.body, "Processed: test_data");
    }

    #[tokio::test]
    async fn test_two_extractors_handler() {
        async fn handler(arg1: MockExtractor, arg2: MockExtractor) -> MockResponse {
            MockResponse(format!("Processed: {} and {}", arg1.0, arg2.0))
        }

        let request = Request::new(Frames::Text, Bytes::from("test_data"));

        let handler_service = ExtractorHandler {
            handler,
            _marker: PhantomData,
        };

        let response = handler_service.call(request, ()).await;
        assert_eq!(response.body, "Processed: test_data and test_data");
    }

    #[tokio::test]
    async fn test_handler_with_state() {
        async fn handler(arg: MockExtractor) -> MockResponse {
            MockResponse(format!("State processed: {}", arg.0))
        }

        let request = Request::new(Frames::Text, Bytes::from("test_state"));
        let state = Arc::new(42);

        let handler_service = ExtractorHandler {
            handler,
            _marker: PhantomData,
        };

        let response = handler_service.call(request, state).await;
        assert_eq!(response.body, "State processed: test_state");
    }
}
