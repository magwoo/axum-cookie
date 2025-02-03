#![doc = include_str!("../README.md")]
use axum_core::extract::{FromRequestParts, Request};
use axum_core::response::Response;
use cookie_rs::{Cookie, CookieJar};
use http::header::{COOKIE, SET_COOKIE};
use http::request::Parts;
use http::{HeaderValue, StatusCode};
use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tower_layer::Layer;
use tower_service::Service;

pub mod cookie {
    pub use cookie_rs::*;
}

pub mod prelude {
    pub use crate::CookieLayer;
    pub use crate::CookieManager;
    pub use cookie_rs::prelude::*;
}

/// Manages cookies using a thread-safe `CookieJar`.
/// This struct provides methods to add, remove, and retrieve cookies,
/// as well as generate `Set-Cookie` headers for HTTP responses.
#[derive(Clone)]
pub struct CookieManager {
    jar: Arc<Mutex<CookieJar<'static>>>,
}

impl CookieManager {
    /// Creates a new instance of `CookieManager` with the specified cookie jar.
    ///
    /// # Arguments
    /// * `jar` - The initial cookie jar to manage cookies.
    pub fn new(jar: CookieJar<'static>) -> Self {
        Self {
            jar: Arc::new(Mutex::new(jar)),
        }
    }

    /// Adds a cookie to the jar.
    ///
    /// # Arguments
    /// * `cookie` - The cookie to add to the jar.
    pub fn add<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let mut jar = self.jar.lock().unwrap();

        jar.add(cookie);
    }

    /// Adds a cookie to the jar.
    ///
    /// # Arguments
    /// * `cookie` - The cookie to add to the jar.
    ///
    /// > alias for `CookieManager::add`
    pub fn set<C: Into<Cookie<'static>>>(&self, cookie: C) {
        let mut jar = self.jar.lock().unwrap();

        jar.add(cookie);
    }

    /// Removes a cookie from the jar by its name.
    ///
    /// # Arguments
    /// * `name` - The name of the cookie to remove.
    pub fn remove(&self, name: &str) {
        let mut jar = self.jar.lock().unwrap();

        jar.remove(name.to_owned());
    }

    /// Retrieves a cookie from the jar by its name.
    ///
    /// # Arguments
    /// * `name` - The name of the cookie to retrieve.
    ///
    /// # Returns
    /// * `Option<Cookie<'static>>` - The cookie if found, otherwise `None`.
    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        let jar = self.jar.lock().unwrap();

        jar.get(name).cloned()
    }

    /// Returns all cookies in the jar as a set.
    ///
    /// # Returns
    /// * `BTreeSet<Cookie<'static>>` - A set of all cookies currently in the jar.
    pub fn cookie(&self) -> BTreeSet<Cookie<'static>> {
        let jar = self.jar.lock().unwrap();

        jar.cookie().into_iter().cloned().collect()
    }

    /// Generates `Set-Cookie` header value for all cookies in the jar.
    ///
    /// # Returns
    /// * `Vec<String>` - A vector of `Set-Cookie` header string value.
    pub fn as_header_value(&self) -> Vec<String> {
        let jar = self.jar.lock().unwrap();

        jar.as_header_values()
    }
}

impl<S> FromRequestParts<S> for CookieManager {
    type Rejection = (StatusCode, String);

    fn from_request_parts(
        parts: &mut Parts,
        _: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        Box::pin(async move {
            parts
                .extensions
                .get::<Result<Self, Self::Rejection>>()
                .cloned()
                .ok_or((
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "CookieLayer is not initialized".to_string(),
                ))?
        })
    }
}

/// A middleware layer for processing cookies.
/// This layer integrates cookie management into the middleware stack.
#[derive(Clone, Default)]
pub struct CookieLayer {
    strict: bool,
}

impl CookieLayer {
    /// Creates a layer with strict cookie parsing enabled.
    pub fn strict() -> Self {
        Self { strict: true }
    }
}

impl<S> Layer<S> for CookieLayer {
    type Service = CookieMiddleware<S>;

    fn layer(&self, inner: S) -> Self::Service {
        CookieMiddleware {
            strict: self.strict,
            inner,
        }
    }
}

/// Middleware for handling HTTP requests and responses with cookies.
/// This middleware parses cookies from requests and adds `Set-Cookie` headers to responses.
#[derive(Clone)]
pub struct CookieMiddleware<S> {
    strict: bool,
    inner: S,
}

impl<S, ReqBody> Service<Request<ReqBody>> for CookieMiddleware<S>
where
    S: Service<Request<ReqBody>, Response = Response<ReqBody>> + Send + 'static,
    S::Future: Send + 'static,
    ReqBody: Send + 'static,
{
    type Response = S::Response;
    type Error = S::Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send>>;

    fn poll_ready(&mut self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.inner.poll_ready(cx)
    }

    fn call(&mut self, mut req: Request<ReqBody>) -> Self::Future {
        let cookie = req
            .headers()
            .get(COOKIE)
            .map(|h| h.to_str())
            .unwrap_or(Ok(""))
            .map(|c| c.to_owned());

        let manager = cookie
            .map(|cookie| {
                match self.strict {
                    false => CookieJar::parse(cookie),
                    true => CookieJar::parse_strict(cookie),
                }
                .map(CookieManager::new)
                .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
            })
            .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))
            .and_then(|inner| inner);

        req.extensions_mut().insert(manager.clone());

        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut response = fut.await?;

            if let Ok(manager) = manager {
                for cookie in manager.as_header_value() {
                    response
                        .headers_mut()
                        .append(SET_COOKIE, HeaderValue::from_str(&cookie).unwrap());
                }
            }

            Ok(response)
        })
    }
}
