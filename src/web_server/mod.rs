mod http_server_context;

use core::str::FromStr;

use defmt::*;
use embassy_executor::Spawner;
use embassy_net::Stack;
use nanofish::{
    Error, HttpHandler, HttpMethod, HttpRequest, HttpResponse, HttpResponseBufferRef,
    HttpResponseBuilder, HttpServer, ServerTimeouts, StatusCode, WebSocket, WebSocketError,
    WebSocketRead, WebSocketState, WebSocketWrite,
};

use crate::configuration::WiFiSettings;
use crate::rtc::*;
use crate::shared_resources::SharedResources;
use crate::{reset, units::TimeExt as _};
use http_server_context::HttpServerContext;

pub use nanofish::HttpServerBuffers;

// Get version from Cargo.toml at compile time
const VERSION: &str = env!("CARGO_PKG_VERSION");

//const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
const MAIN_CONFIGURATION_HTML_GZ: &[u8] = include_bytes!("./web/main_configuration.html.gz");

pub struct HttpConfigServer {
    context: HttpServerContext,
    http_server: HttpServer,
}

impl HttpConfigServer {
    pub fn new(spawner: Spawner, shared: &'static SharedResources) -> Self {
        let timeouts = ServerTimeouts::default();
        //timeouts.read_timeout = 1;

        let http_server: HttpServer = HttpServer::new(80).with_timeouts(timeouts);
        Self {
            context: HttpServerContext::new(spawner, shared),
            http_server,
        }
    }

    pub fn with_auto_close_connection(mut self, auto_close: bool) -> Self {
        self.http_server = self.http_server.with_auto_close_connection(auto_close);
        self
    }

    pub async fn run<
        const SOCKETS: usize,
        const RX_SIZE: usize,
        const TX_SIZE: usize,
        const REQ_SIZE: usize,
        const MAX_RESPONSE_SIZE: usize,
    >(
        &mut self,
        stack: Stack<'_>,
        buffers: &mut HttpServerBuffers<SOCKETS, RX_SIZE, TX_SIZE, REQ_SIZE, MAX_RESPONSE_SIZE>,
    ) -> ! {
        self.http_server
            .serve(stack, buffers, HttpConfigHandler::new(&self.context))
            .await
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

fn trace_headers(request: &HttpRequest<'_>) {
    debug!("Request header");
    for header in request.headers.iter() {
        debug!(" - : {}: {}", header.name, header.value);
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
            //trace_headers(request);

            return HttpResponseBuilder::new(response_buffer)
                .with_status(StatusCode::Ok)?
                .with_compressed_page(MAIN_CONFIGURATION_HTML_GZ);
        }

        let Some(api) = request.path.strip_prefix("/api/") else {
            return HttpResponseBuilder::new(response_buffer)
                .with_status(StatusCode::NotFound)?
                .with_plain_text_body("Not Found");
        };

        if request.method == HttpMethod::OPTIONS {
            trace_headers(request);
        }

        match (request.method, api) {
            (HttpMethod::OPTIONS, command) => {
                //TODO: Implement more strict header checking
                debug!("Serving {} preflight request", command);
                HttpResponseBuilder::new(response_buffer).preflight_response()
            }
            (HttpMethod::GET, "version") => {
                debug!("Serving version info");
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body(VERSION)
            }
            (HttpMethod::GET, "reboot") => {
                info!("Serving reboot request");
                reset::deferred_system_reset(self.context.spawner(), 1.s());
                // The reset function does not return, but we provide a response for completeness
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body("System is resetting...")
            }
            (HttpMethod::GET, "wifi_config") => {
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
            (HttpMethod::POST, "set_wifi_config") => {
                debug!("Serving set configuration request");
                //TODO: Implement data integrity checks
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
                match self.context.configuration_storage().save().await {
                    Ok(_) => HttpResponseBuilder::new(response_buffer)
                        .with_status(StatusCode::Ok)?
                        .with_plain_text_body("WiFi configuration updated"),
                    Err(e) => {
                        error!("Failed to save configuration: {:?}", e);
                        HttpResponseBuilder::new(response_buffer)
                            .with_status(StatusCode::InternalServerError)?
                            .with_plain_text_body("Failed to save WiFi configuration")
                    }
                }
            }
            (HttpMethod::GET, "date_time") => {
                debug!("Serving date_time request");

                let mut rtc = self.context.rtc().lock().await;
                let datetime = rtc.datetime().await.map_err(|e| {
                    error!("RTC datetime read error: {}", e);
                    Error::NoResponse
                })?;

                let mut date_time_str = heapless::String::<64>::new();

                //ISO 8601 format could be used as well
                //"1995-12-17T03:24:00Z"
                core::fmt::write(&mut date_time_str, format_args!("{}", datetime))
                    .map_err(|_| Error::NoResponse)?;
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body(&date_time_str)
            }

            (HttpMethod::POST, "set_date_time") => {
                debug!("Serving set_date_time request");
                let date_time_str = core::str::from_utf8(request.body).map_err(|_| {
                    error!("Invalid UTF-8 in request body");
                    Error::NoResponse
                })?;
                let date_time = NaiveDateTime::from_str(date_time_str).map_err(|_| {
                    error!("Invalid date time format: {}", date_time_str);
                    Error::NoResponse
                })?;

                let mut rtc = self.context.rtc().lock().await;
                rtc.set_datetime(&date_time).await.map_err(|e| {
                    error!("RTC datetime set error: {}", e);
                    Error::NoResponse
                })?;

                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body("Date and time updated")
            }

            _ => HttpResponseBuilder::new(response_buffer)
                .with_status(StatusCode::NotFound)?
                .with_plain_text_body("Not Found"),
        }
    }

    async fn handle_websocket_connection<'h>(
        &mut self,
        _request: &HttpRequest<'_>,
        mut web_socket: WebSocket<'h, '_>,
    ) -> Result<(), ()> {
        let mut buffer = [0u8; 128];
        let len = web_socket.read(&mut buffer).await.map_err(|_| ())?;
        let str = core::str::from_utf8(&buffer[..len]).unwrap_or("<invalid utf-8>");
        defmt::info!("Received WebSocket frame: {}", str);
        web_socket
            .write_all(b"Hello from WebSocket!")
            .await
            .map_err(|_| ())?;

        web_socket.close().await.map_err(|_| ())?;

        Ok(()) // Close the connection immediately
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
