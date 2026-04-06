mod http_server_context;

use core::mem::MaybeUninit;
use core::str::FromStr;

use bump_into::BumpInto;
use defmt_or_log::{self as log};
use embassy_executor::Spawner;
use embassy_net::Stack;
use nanofish::{
    Error, HttpHandler, HttpMethod, HttpRequest, HttpResponse, HttpResponseBufferRef, HttpResponseBuilder, HttpServer,
    ServerTimeouts, SocketBuffers, StatusCode, WebSocket, WebSocketRead, WebSocketWrite,
};

use crate::board::*;
use crate::configuration::WiFiSettings;
use crate::rtc::*;
use crate::shared_resources::SharedResources;
use crate::ws2812b_led_controller::*;
use crate::{reset, units::TimeExt as _};
use http_server_context::HttpServerContext;

// Get version from Cargo.toml at compile time
const VERSION: &str = env!("CARGO_PKG_VERSION");

//const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
const MAIN_CONFIGURATION_HTML_GZ: &[u8] = include_bytes!("./web/main_configuration.html.gz");

/// RX buffer size for each socket in the pool
const SOCKET_RX_SIZE: usize = 256;
/// TX buffer size for each socket in the pool
const SOCKET_TX_SIZE: usize = 256;

// Maximum request/response size for a single HTTP server worker
const WORKER_BUFFER_SIZE: usize = 8192;

// Port for the HTTP server to listen on
const HTTP_SERVER_PORT: u16 = 80;

pub struct HttpConfigServer<'buffer, const SOCKETS: usize> {
    http_server: HttpServer<'buffer, SOCKETS>,
}

#[allow(dead_code)]
impl<'stack, const SOCKETS: usize> HttpConfigServer<'stack, SOCKETS> {
    pub const MIN_SOCKET_POOL_BUFFER_SIZE: usize = (SOCKET_RX_SIZE + SOCKET_TX_SIZE) * SOCKETS;
    pub const MIN_WORKER_BUFFER_SIZE: usize = WORKER_BUFFER_SIZE;

    pub fn new<'buffer>(server_allocator: &mut BumpInto<'buffer>, stack: Stack<'stack>) -> Self
    where
        'buffer: 'stack,
    {
        let timeouts = ServerTimeouts::default();

        let socket_buffers = server_allocator
            .alloc_with(|| [const { SocketBuffers::<SOCKET_RX_SIZE, SOCKET_TX_SIZE>::new() }; SOCKETS])
            .unwrap_or_else(|_| {
                panic!(
                    "Not enough memory to store the socket pool buffers. Required: {} bytes but only {} bytes available.",
                    SOCKETS * core::mem::size_of::<SocketBuffers<SOCKET_RX_SIZE, SOCKET_TX_SIZE>>(),
                    server_allocator.available_bytes()
                )
            });

        let http_server =
            HttpServer::new::<SOCKET_RX_SIZE, SOCKET_TX_SIZE>(socket_buffers, stack, HTTP_SERVER_PORT, timeouts);
        Self { http_server }
    }

    pub fn with_auto_close_connection(mut self, auto_close: bool) -> Self {
        self.http_server = self.http_server.with_auto_close_connection(auto_close);
        self
    }

    pub async fn run(
        &self,
        worker_memory_buf: &mut [MaybeUninit<u8>],
        spawner: Spawner,
        shared: &'static SharedResources,
    ) -> ! {
        let context = HttpServerContext::new(spawner, shared);
        let mut handler = HttpConfigHandler::new(&context);
        self.http_server.serve::<_>(worker_memory_buf, &mut handler).await
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

    async fn handle_request_impl(
        &mut self,
        request: &HttpRequest<'_>,
        response_buffer: HttpResponseBufferRef<'_>,
    ) -> Result<HttpResponse, Error> {
        if request.path == "/" {
            // Show main page
            log::debug!("Serving main configuration page");

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
                log::debug!("Serving {} preflight request", command);
                HttpResponseBuilder::new(response_buffer).preflight_response()
            }
            (HttpMethod::GET, "version") => {
                log::debug!("Serving version info");
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body(VERSION)
            }
            (HttpMethod::GET, "reboot") => {
                log::info!("Serving reboot request");
                reset::deferred_system_reset(self.context.spawner(), 1.s());
                // The reset function does not return, but we provide a response for completeness
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body("System is resetting...")
            }
            (HttpMethod::GET, "wifi_config") => {
                log::debug!("Serving configuration request");
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
                log::debug!("Serving set configuration request");
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
                        log::error!("Failed to save configuration: {:?}", e);
                        HttpResponseBuilder::new(response_buffer)
                            .with_status(StatusCode::InternalServerError)?
                            .with_plain_text_body("Failed to save WiFi configuration")
                    }
                }
            }
            (HttpMethod::GET, "date_time") => {
                log::debug!("Serving date_time request");

                let mut rtc = self.context.rtc().lock().await;
                let datetime = rtc.datetime().await.map_err(|e| {
                    log::error!("RTC datetime read error: {}", e);
                    Error::NoResponse
                })?;

                let mut date_time_str = heapless::String::<64>::new();

                //ISO 8601 format could be used as well
                //"1995-12-17T03:24:00Z"
                core::fmt::write(&mut date_time_str, format_args!("{}", datetime)).map_err(|_| Error::NoResponse)?;
                HttpResponseBuilder::new(response_buffer)
                    .with_status(StatusCode::Ok)?
                    .with_plain_text_body(&date_time_str)
            }

            (HttpMethod::POST, "set_date_time") => {
                log::debug!("Serving set_date_time request");
                let date_time_str = core::str::from_utf8(request.body).map_err(|_| {
                    log::error!("Invalid UTF-8 in request body");
                    Error::NoResponse
                })?;
                let date_time = NaiveDateTime::from_str(date_time_str).map_err(|_| {
                    log::error!("Invalid date time format: {}", date_time_str);
                    Error::NoResponse
                })?;

                let mut rtc = self.context.rtc().lock().await;
                rtc.set_datetime(&date_time).await.map_err(|e| {
                    log::error!("RTC datetime set error: {}", e);
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

    async fn handle_websocket_connection_impl<'h>(
        &mut self,
        _request: &HttpRequest<'_>,
        mut web_socket: WebSocket<'h, '_>,
    ) -> Result<(), ()> {
        let mut buffer = [0u8; 128];
        let len = web_socket.read(&mut buffer).await.map_err(|_| ())?;
        let str = core::str::from_utf8(&buffer[..len]).unwrap_or("<invalid utf-8>");
        log::info!("Received WebSocket frame: {}", str);
        web_socket.write_all(b"Hello from WebSocket!").await.map_err(|_| ())?;

        web_socket.close().await.map_err(|_| ())?;

        Ok(()) // Close the connection immediately
    }
}

fn trace_headers(request: &HttpRequest<'_>) {
    log::debug!("Request header");
    for header in request.headers.iter() {
        log::debug!(" - : {}: {}", header.name, header.value);
    }
}

impl<'a> HttpHandler for HttpConfigHandler<'a> {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
        response_buffer: HttpResponseBufferRef<'_>,
    ) -> Result<HttpResponse, Error> {
        self.context
            .shared_resources()
            .led_controller
            .set_animation(LED_1, LedAnimation::Decay(color::ORANGE, 300))
            .await;

        let res = self.handle_request_impl(request, response_buffer).await;

        res
    }

    async fn handle_websocket_connection<'h>(
        &mut self,
        _request: &HttpRequest<'_>,
        web_socket: WebSocket<'h, '_>,
    ) -> Result<(), ()> {
        self.context
            .shared_resources()
            .led_controller
            .set_animation(LED_1, LedAnimation::Decay(color::ORANGE, 300))
            .await;

        let res = self.handle_websocket_connection_impl(_request, web_socket).await;

        res
    }
}

fn to_response<T>(response_buffer: HttpResponseBufferRef<'_>, value: &T) -> Result<HttpResponse, Error>
where
    T: serde::Serialize,
{
    HttpResponseBuilder::new(response_buffer)
        .with_status(StatusCode::Ok)?
        .with_header("Content-Type", "application/json")?
        .with_body_filler(|buf| {
            serde_json_core::to_slice(value, buf).map_err(|e| {
                log::error!("Serialization error: {}", e);
                Error::NoResponse
            })
        })
}

fn from_request<'de, T>(request: &HttpRequest<'de>) -> Result<T, nanofish::Error>
where
    T: serde::Deserialize<'de>,
{
    let (value, _) = serde_json_core::from_slice(request.body).map_err(|e| {
        log::error!("Deserialization error: {}", e);
        nanofish::Error::NoResponse
    })?;

    Ok(value)
}

// fn from_http_response(request: &HttpRequest<'de>) -> Result<T, nanofish::Error> {
//     let (value, _) = serde_json_core::from_slice(request.body).map_err(|e| {
//         log::error!("Deserialization error: {}", e);
//         nanofish::Error::NoResponse
//     })?;

//     Ok(value)
// }
