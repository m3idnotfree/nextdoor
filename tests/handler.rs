use std::{marker::PhantomData, sync::Arc};

use bytes::Bytes;

use nextdoor::{
    error::ExtractError,
    extract::{FromMesasge, State},
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
    async fn handler_withstate(_arg: MockExtractor, state: State<Arc<i32>>) -> MockResponse {
        MockResponse(format!("State processed: {}", state.0))
    }

    let request = Request::new(Frames::Text, Bytes::from("test_state"));
    let state = Arc::new(42);

    let handler_service = ExtractorHandler {
        handler,
        _marker: PhantomData,
    };

    let response = handler_service.call(request.clone(), state.clone()).await;
    assert_eq!(response.body, "State processed: test_state");

    let handler_service = ExtractorHandler {
        handler: handler_withstate,
        _marker: PhantomData,
    };

    let response = handler_service.call(request, state).await;
    assert_eq!(response.body, "State processed: 42");
}
