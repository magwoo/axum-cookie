use axum::extract::FromRequestParts;
use axum::http::request::Parts;
use axum::http::{HeaderValue, Request, Response, StatusCode};
use cookie_rs::{Cookie, CookieJar};
use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll};
use tower::{Layer, Service};

#[derive(Clone)]
pub struct CookieManager {
    jar: Arc<Mutex<CookieJar<'static>>>,
}

impl CookieManager {
    pub fn new(jar: CookieJar<'static>) -> Self {
        Self {
            jar: Arc::new(Mutex::new(jar)),
        }
    }

    pub fn add(&self, cookie: Cookie<'static>) {
        let mut jar = self.jar.lock().unwrap();

        jar.add(cookie);
    }

    pub fn remove(&self, name: &str) {
        let mut jar = self.jar.lock().unwrap();

        jar.remove(name.to_owned());
    }

    pub fn get(&self, name: &str) -> Option<Cookie<'static>> {
        let jar = self.jar.lock().unwrap();

        jar.get(name).cloned()
    }

    pub fn cookie(&self) -> BTreeSet<Cookie<'static>> {
        let jar = self.jar.lock().unwrap();

        jar.cookie().into_iter().cloned().collect()
    }

    pub fn into_set_cookie_headers(&self) -> Vec<String> {
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
                    "Can't extract cookies. Is `CookieLayer` enabled?".to_string(),
                ))?
        })
    }
}

#[derive(Clone, Default)]
pub struct CookieLayer {
    strict: bool,
}

impl CookieLayer {
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
            .get_all(axum::http::header::COOKIE)
            .iter()
            .flat_map(|value| value.to_str().ok())
            .collect::<String>();

        let manager = match self.strict {
            false => CookieJar::parse(cookie),
            true => CookieJar::parse_strict(cookie),
        }
        .map(CookieManager::new)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()));

        req.extensions_mut().insert(manager.clone());

        let fut = self.inner.call(req);

        Box::pin(async move {
            let mut response = fut.await?;

            if let Ok(manager) = manager {
                for cookie in manager.into_set_cookie_headers() {
                    response.headers_mut().append(
                        axum::http::header::SET_COOKIE,
                        HeaderValue::from_str(&cookie).unwrap(),
                    );
                }
            }

            Ok(response)
        })
    }
}
