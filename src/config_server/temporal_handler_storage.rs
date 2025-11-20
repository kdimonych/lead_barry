use nanofish::{HttpRequest, HttpResponse};

use super::http_main_page_handler::MainPageHandler;
use super::http_server_context::HttpServerContext;
use super::temporal_handler::TemporalHttpHandler;

pub enum TemporalHandlerStorage {
    None,
    MainPage(MainPageHandler),
}

impl TemporalHandlerStorage {
    /// Handle the request using the specified handler type.
    /// - The handler will be instantiated and stored in the enum which gives the handler and its
    ///   byproducts a temporal lifetime enough to handle the request.
    /// - The handler must implement `Default` to be instantiated.
    ///
    /// Usage:
    /// ```rust
    /// self.active_handler.handle_request::<MainPageHandler>(request, context).await;
    /// ```
    pub async fn handle_request<Handler>(
        &mut self,
        request: &HttpRequest<'_>,
        context: &'_ HttpServerContext,
    ) -> Result<HttpResponse<'_>, nanofish::Error>
    where
        Handler: TemporalHttpHandler + Into<TemporalHandlerStorage> + Default,
    {
        // Instatntiate the handler and replace self with it
        // This gives the handler and its byproducts a temporal lifetime enough to handle the request
        *self = core::mem::replace(self, Handler::default().into());

        // Delegate the request handling to the active handler
        match self {
            TemporalHandlerStorage::MainPage(handler) => {
                handler.handle_request(request, context).await
            }
            TemporalHandlerStorage::None => unreachable!(),
        }
    }
}

impl From<MainPageHandler> for TemporalHandlerStorage {
    fn from(handler: MainPageHandler) -> Self {
        TemporalHandlerStorage::MainPage(handler)
    }
}
