mod http_server_context;

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::Stack;
use nanofish::{
    Error, HttpHandler, HttpRequest, HttpResponse, HttpResponseBufferRef, HttpResponseBuilder,
    HttpServer, StatusCode,
};

use crate::configuration::{ConfigurationStorage, WiFiSettings};
use crate::{reset, units::TimeExt as _};
use http_server_context::HttpServerContext;

// Get version from Cargo.toml at compile time
const VERSION: &str = env!("CARGO_PKG_VERSION");

//const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
const MAIN_CONFIGURATION_HTML_GZ: &[u8] = include_bytes!("./web/main_configuration.html.gz");

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
}

impl<'a> HttpConfigHandler<'a> {
    fn new(context: &'a HttpServerContext) -> Self {
        Self { context }
    }
}

impl<'a> HttpHandler for HttpConfigHandler<'a> {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
        response_buffer: HttpResponseBufferRef<'_>,
    ) -> Result<HttpResponse, Error> {
        if request.path == "/" {
            // Show main page
            debug!("Serving main configuration page");

            // return HttpResponseBuilder::new(response_buffer)
            //     .with_page(b"<h1>Hello from nanofish HTTP server!</h1>");
            return HttpResponseBuilder::new(response_buffer)
                .with_compressed_page(MAIN_CONFIGURATION_HTML_GZ);
        }

        let Some(api) = request.path.strip_prefix("/api/") else {
            return HttpResponseBuilder::new(response_buffer)
                .with_status(StatusCode::NotFound)?
                .with_plain_text_body("Not Found");
        };

        match api {
            "version" => {
                debug!("Serving version info");
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body(VERSION)
            }
            "reset" => {
                info!("Serving reset request");
                reset::deferred_system_reset(self.context.spawner(), 1.s());
                // The reset function does not return, but we provide a response for completeness
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body("System is resetting...")
            }
            "wifi_config" => {
                debug!("Serving configuration request");
                let mut wifi_settings = self
                    .context
                    .configuration_storage()
                    .get_settings()
                    .await
                    .network_settings
                    .wifi_settings;

                // Clear password before sending
                if let Some(psw) = wifi_settings.password.as_mut() {
                    psw.clear()
                }

                to_response(response_buffer, &wifi_settings)
            }
            "set_wifi_config" => {
                debug!("Serving set configuration request");
                let mut wifi_settings: WiFiSettings = from_request(request)?;
                if wifi_settings.password.is_none() {
                    // Preserve existing password if not provided
                    let current_settings = self
                        .context
                        .configuration_storage()
                        .get_settings()
                        .await
                        .network_settings
                        .wifi_settings;
                    wifi_settings.password = current_settings.password;
                }
                self.context
                    .configuration_storage()
                    .modify_settings(|settings| {
                        settings.network_settings.wifi_settings = wifi_settings;
                    })
                    .await;
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body("WiFi configuration updated")
            }
            _ => HttpResponseBuilder::new(response_buffer)
                .with_status(StatusCode::NotFound)?
                .with_plain_text_body("Not Found"),
        }
    }
}

fn to_response<T>(
    response_buffer: HttpResponseBufferRef<'_>,
    value: &T,
) -> Result<HttpResponse, Error>
where
    T: serde::Serialize,
{
    HttpResponseBuilder::new(response_buffer)
        .with_status(StatusCode::Ok)?
        .with_header("Content-Type", "application/json")?
        .with_body_filler(|buf| {
            serde_json_core::to_slice(value, buf).map_err(|e| {
                error!("Serialization error: {}", e);
                Error::NoResponse
            })
        })
}

fn from_request<'de, T>(request: &HttpRequest<'de>) -> Result<T, nanofish::Error>
where
    T: serde::Deserialize<'de>,
{
    let (value, _) = serde_json_core::from_slice(request.body).map_err(|e| {
        error!("Deserialization error: {}", e);
        nanofish::Error::NoResponse
    })?;

    Ok(value)
}

// fn from_http_response(request: &HttpRequest<'de>) -> Result<T, nanofish::Error> {
//     let (value, _) = serde_json_core::from_slice(request.body).map_err(|e| {
//         error!("Deserialization error: {}", e);
//         nanofish::Error::NoResponse
//     })?;

//     Ok(value)
// }
