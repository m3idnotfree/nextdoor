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
    T: Send + Sync + 'static,
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
