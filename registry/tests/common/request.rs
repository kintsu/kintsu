//! Fluent request builder for test HTTP requests

use actix_http::Request;
use actix_web::{
    dev::{Service, ServiceResponse},
    http::Method,
    test::TestRequest,
};
use serde::Serialize;

use super::{TestRegistryCtx, TestResponse};

/// Fluent builder for constructing test HTTP requests
pub struct RequestBuilder<'ctx> {
    ctx: &'ctx TestRegistryCtx,
    method: Method,
    path: String,
    headers: Vec<(String, String)>,
    body: Option<Vec<u8>>,
    query_params: Vec<(String, String)>,
}

impl<'ctx> RequestBuilder<'ctx> {
    /// Create a GET request builder
    pub fn get(
        ctx: &'ctx TestRegistryCtx,
        path: &str,
    ) -> Self {
        Self::new(ctx, Method::GET, path)
    }

    /// Create a POST request builder
    pub fn post(
        ctx: &'ctx TestRegistryCtx,
        path: &str,
    ) -> Self {
        Self::new(ctx, Method::POST, path)
    }

    /// Create a DELETE request builder
    pub fn delete(
        ctx: &'ctx TestRegistryCtx,
        path: &str,
    ) -> Self {
        Self::new(ctx, Method::DELETE, path)
    }

    /// Create a PUT request builder
    pub fn put(
        ctx: &'ctx TestRegistryCtx,
        path: &str,
    ) -> Self {
        Self::new(ctx, Method::PUT, path)
    }

    fn new(
        ctx: &'ctx TestRegistryCtx,
        method: Method,
        path: &str,
    ) -> Self {
        Self {
            ctx,
            method,
            path: path.to_string(),
            headers: vec![],
            body: None,
            query_params: vec![],
        }
    }

    /// Set Bearer token authorization header
    pub fn bearer(
        mut self,
        token: &str,
    ) -> Self {
        self.headers
            .push(("Authorization".to_string(), format!("Bearer {}", token)));
        self
    }

    /// Set JSON body and Content-Type header
    pub fn json<T: Serialize>(
        mut self,
        body: &T,
    ) -> Self {
        self.body = Some(serde_json::to_vec(body).unwrap());
        self.headers
            .push(("Content-Type".to_string(), "application/json".to_string()));
        self
    }

    /// Set raw bytes body
    pub fn bytes(
        mut self,
        body: Vec<u8>,
    ) -> Self {
        self.body = Some(body);
        self
    }

    /// Set Content-Type header
    pub fn content_type(
        mut self,
        content_type: &str,
    ) -> Self {
        self.headers
            .push(("Content-Type".to_string(), content_type.to_string()));
        self
    }

    /// Set raw string body
    pub fn body(
        mut self,
        body: &str,
    ) -> Self {
        self.body = Some(body.as_bytes().to_vec());
        self
    }

    /// Add a query parameter
    pub fn query(
        mut self,
        key: &str,
        value: &str,
    ) -> Self {
        self.query_params
            .push((key.to_string(), value.to_string()));
        self
    }

    /// Add a custom header
    pub fn header(
        mut self,
        key: &str,
        value: &str,
    ) -> Self {
        self.headers
            .push((key.to_string(), value.to_string()));
        self
    }

    /// Send the request and return TestResponse
    pub async fn send(self) -> TestResponse {
        let app = self.ctx.app().await;

        let mut path = self.path.clone();
        if !self.query_params.is_empty() {
            let query_string: Vec<String> = self
                .query_params
                .iter()
                .map(|(k, v)| format!("{}={}", k, v))
                .collect();
            path = format!("{}?{}", path, query_string.join("&"));
        }

        let mut req = TestRequest::default()
            .method(self.method)
            .uri(&path);

        for (key, value) in &self.headers {
            req = req.insert_header((key.as_str(), value.as_str()));
        }

        if let Some(body) = self.body {
            req = req.set_payload(body);
        }

        let req: Request = req.to_request();
        let resp = app.call(req).await.unwrap();

        TestResponse::new(resp).await
    }
}
