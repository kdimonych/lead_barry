mod configuration;
mod http_main_page_handler;
mod http_server_context;
mod temporal_handler;
mod temporal_handler_storage;

use defmt::info;
use embassy_executor::Spawner;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{HttpHandler, HttpRequest, HttpResponse, HttpServer, ResponseBody, StatusCode};

// Get version from Cargo.toml at compile time
const VERSION: &str = env!("CARGO_PKG_VERSION");

use crate::configuration::ConfigurationStorage;
use crate::web_server::http_main_page_handler::MainPageHandler;
use crate::{reset, units::TimeExt as _};
use http_server_context::HttpServerContext;
use temporal_handler_storage::TemporalHandlerStorage;

const RX_SIZE: usize = 2048;
const TX_SIZE: usize = 2048;
const REQ_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 8192;

pub struct HttpConfigServer {
    context: HttpServerContext,
    http_server: HttpServer<RX_SIZE, TX_SIZE, REQ_SIZE, MAX_RESPONSE_SIZE>,
}

impl HttpConfigServer {
    pub fn new(
        spawner: Spawner,
        configuration_storage: &'static ConfigurationStorage<'static>,
    ) -> Self {
        let http_server = HttpServer::new(80);
        Self {
            context: HttpServerContext::new(spawner, configuration_storage),
            http_server,
        }
    }

    pub async fn run(&mut self, stack: Stack<'_>) -> ! {
        self.http_server
            .serve(stack, HttpConfigHandler::new(&self.context))
            .await;
    }
}

// Create a simple request handler
struct HttpConfigHandler<'a> {
    context: &'a HttpServerContext,
    active_handler: TemporalHandlerStorage,
}

impl<'a> HttpConfigHandler<'a> {
    fn new(context: &'a HttpServerContext) -> Self {
        Self {
            context,
            active_handler: TemporalHandlerStorage::None,
        }
    }
}

impl<'a> HttpHandler for HttpConfigHandler<'a> {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        if request.path == "/" {
            // Show main page
            info!("Serving main configuration page");

            return self
                .active_handler
                .handle_request::<MainPageHandler>(request, self.context)
                .await;
        }

        let Some(api) = request.path.strip_prefix("/api/") else {
            return Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            });
        };

        match api {
            "version" => {
                info!("Serving version info");
                Ok(HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text(VERSION),
                })
            }

            "reset" => {
                reset::deferred_system_reset(self.context.spawner(), 1.s());
                // The reset function does not return, but we provide a response for completeness
                Ok(HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text("System is resetting..."),
                })
            }
            _ => Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Invalid API endpoint"),
            }),
        }
    }
}
