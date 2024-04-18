use actix_web::{
    dev::{forward_ready, Service, ServiceRequest, ServiceResponse, Transform},
    error::ErrorUnauthorized,
    Error, HttpMessage,
};
use actix_web_httpauth::extractors::basic::BasicAuth;
use futures_util::{future::LocalBoxFuture, FutureExt};
use std::{
    future::{ready, Ready},
    rc::Rc,
};
use uuid::Uuid;

pub struct BasicAuthentication;

// Middleware factory is `Transform` trait
// `S` - type of the next service
// `B` - type of response's body
impl<S, B> Transform<S, ServiceRequest> for BasicAuthentication
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = BasicAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ready(Ok(BasicAuthMiddleware {
            service: Rc::new(service),
        }))
    }
}

pub struct BasicAuthMiddleware<S> {
    service: Rc<S>,
}

impl<S, B> Service<ServiceRequest> for BasicAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = LocalBoxFuture<'static, Result<Self::Response, Self::Error>>;

    forward_ready!(service);

    fn call(&self, mut req: ServiceRequest) -> Self::Future {
        let srv = self.service.clone();

        async move {
            let auth = req
                .extract::<BasicAuth>()
                .await
                .ok()
                .ok_or(ErrorUnauthorized("No credentials given"))?;

            let username = auth
                .user_id()
                .parse::<Uuid>()
                .ok()
                .ok_or(ErrorUnauthorized("Invalid username"))?;

            let password = auth
                .password()
                .ok_or(ErrorUnauthorized("No password given"))?;

            if password != "🏳️‍⚧️" {
                return Err(ErrorUnauthorized("Invalid password"));
            }

            req.extensions_mut().insert(username);

            Ok(srv.call(req).await?)
        }
        .boxed_local()
    }
}
