# Axum Cookie Middleware
This library provides a middleware layer for integrating cookie management into Axum applications.
It allows parsing cookies from incoming requests, managing cookies
and setting `Set-Cookie` headers in HTTP responses.

## Features
- Middleware integration with Axum.
- Flexible cookie parsing (strict or lenient mode).
- Thread-safe cookie management.
- Automatic handling of `Set-Cookie` headers.

## Usage
Add the library to your `Cargo.toml`:
```diff
[dependencies]
...
+ axum-cookie = "0.1.0"
```

### Example: Basic Integration
```
use axum::{Router, routing::get};
use axum_cookie::prelude::*;
async fn handler(cookie: CookieManager) -> &'static str {
    // Retrieve a cookie
    if let Some(cookie) = cookie.get("session") {
        println!("Cookie value: {}", cookie.value());
    }
    // Add a cookie
    cookie.add(Cookie::new("session", "abc123"));
    "Hello, cookies!"
}
let app: Router<()> = Router::new()
    .route("/", get(handler))
    .layer(CookieLayer::default());
```

### Example: Strict Cookie Parsing
```rust
use axum::{Router, routing::get};
use axum_cookie::CookieLayer;
let app: Router<()> = Router::new()
    .route("/", get(|| async { "Strict mode enabled" }))
    .layer(CookieLayer::strict());
```

## API

### CookieManager
- `CookieManager::new` - Creates a new cookie manager with a specified `CookieJar`.
- `CookieManager::add` - Adds a cookie to the jar.
- `CookieManager::remove` - Removes a cookie by its name.
- `CookieManager::get` - Retrieves a cookie by its name.
- `CookieManager::cookie` - Returns all cookies in the jar.
- `CookieManager::into_set_cookie_headers` - Generates `Set-Cookie` header value for all cookies in the jar.

### CookieLayer
- `CookieLayer::default` - Creates a layer with lenient cookie parsing.
- `CookieLayer::strict` - Creates a layer with strict cookie parsing.

### CookieMiddleware
- Handles parsing cookies from requests.
- Adds `Set-Cookie` headers to responses.

## Contributing
Contributions are welcome! Please open an issue or submit a pull request on GitHub.

## License
This project is licensed under the MIT License.
