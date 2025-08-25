//! Basic example of making use of `axum-cookie` in `lenient mode`.
//!
//! Sets `counter` cookie on first request, and increments it on subsequent requests.
//!
//! Run with
//!
//! ```not_rust
//! cargo run -p example-hello-cookie
//! ```

use axum::Router;
use axum::routing::get;
use axum_cookie::prelude::*;

#[tokio::main]
async fn main() {
    let app = Router::new()
        .route("/", get(handler))
        .layer(CookieLayer::default());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:3000")
        .await
        .unwrap();
    println!("Listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, app).await.unwrap();
}

async fn handler(cookie: CookieManager) -> String {
    let mut num = 0;
    // Get cookie
    if let Some(cookie) = cookie.get("counter") {
        println!("Cookie.get(counter): {cookie}");
        num = cookie.value().parse::<u32>().unwrap();
    } else {
        println!("Cookie.get(counter): None");
    }
    num += 1;

    // Set cookie
    cookie.add(Cookie::new("counter", num.to_string()));
    //Response
    format!("Hello from axum-cookie! Counter: {num}")
}
