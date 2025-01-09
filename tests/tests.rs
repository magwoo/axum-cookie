use axum::http::StatusCode;
use axum_cookie::CookieLayer;
use axum_cookie::CookieManager;
use cookie_rs::prelude::*;

#[tokio::test]
async fn test_cookie_manager_add_get_remove() {
    use cookie_rs::Cookie;

    let jar = CookieJar::default();
    let manager = CookieManager::new(jar);

    let cookie = Cookie::new("key", "value");
    manager.add(cookie.clone());

    assert_eq!(manager.get("key"), Some(cookie.clone()));
    manager.remove("key");
    assert!(manager.get("key").is_none());
}

#[tokio::test]
async fn test_cookie_middleware_parsing() {
    use axum::{body::Body, http::Request, routing::get, Router};
    use tower::ServiceExt;

    let app = Router::new()
        .route(
            "/",
            get(|manager: CookieManager| async move {
                if manager.get("test").is_some_and(|c| c.value() == "value") {
                    return StatusCode::OK;
                }

                StatusCode::BAD_REQUEST
            }),
        )
        .layer(CookieLayer::default());

    // Создаем запрос с заголовком Cookie
    let request = Request::builder()
        .uri("/")
        .header("Cookie", "test=value")
        .body(Body::empty())
        .unwrap();

    // Выполняем запрос
    let response = app.oneshot(request).await.unwrap();

    assert_eq!(response.status(), StatusCode::OK);
}
