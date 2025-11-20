use super::http_server_context::HttpServerContext;
use nanofish::{HttpRequest, HttpResponse};

/// A trait for handling HTTP requests with access to the server context
/// and configuration storage.
/// A temporal. handler will live as long as the request is being processed.
pub trait TemporalHttpHandler {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
        context: &'_ HttpServerContext,
    ) -> Result<HttpResponse<'_>, nanofish::Error>;
}
