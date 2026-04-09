mod http_server_context;

use core::mem::MaybeUninit;
use core::str::FromStr;

use bump_into::BumpInto;
use defmt_or_log::{self as log};
use embassy_executor::Spawner;
use embassy_net::Stack;
use nanofish::{
    Error, HttpHandler, HttpMethod, HttpRequest, HttpResponseBuilder, HttpServer, HttpWriteSocket, ServerTimeouts,
    SocketBuffers, StatusCode, WebSocket, WebSocketRead,
};
use prefix_arena::PrefixArena;

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
        let mut handler = HttpWebAPIHandler::new(&context);
        self.http_server.serve::<_>(worker_memory_buf, &mut handler).await
    }
}

// Create a simple request handler
struct HttpWebAPIHandler<'a> {
    context: &'a HttpServerContext,
}

impl<'a> HttpWebAPIHandler<'a> {
    fn new(context: &'a HttpServerContext) -> Self {
        Self { context }
    }

    async fn api_version<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        _request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        log::debug!("Serving version info");
        HttpResponseBuilder::new(http_socket)
            .with_status(StatusCode::Ok)
            .await?
            .with_plain_text_body(VERSION)
            .await
    }

    async fn api_reboot<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        _request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        log::info!("Serving reboot request");
        reset::deferred_system_reset(self.context.spawner(), 1.s());
        // The reset function does not return, but we provide a response for completeness
        HttpResponseBuilder::new(http_socket)
            .with_status(StatusCode::Ok)
            .await?
            .with_plain_text_body("System is resetting...")
            .await
    }

    async fn api_wifi_config<HttpSocket: HttpWriteSocket>(
        &mut self,
        allocator: &mut PrefixArena<'_>,
        _request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
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

        send_serialized_type(allocator, http_socket, &wifi_settings).await
    }

    async fn api_set_wifi_config<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
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
            Ok(_) => {
                HttpResponseBuilder::new(http_socket)
                    .with_status(StatusCode::Ok)
                    .await?
                    .with_plain_text_body("WiFi configuration updated")
                    .await
            }
            Err(e) => {
                log::error!("Failed to save configuration: {:?}", e);
                HttpResponseBuilder::new(http_socket)
                    .with_status(StatusCode::InternalServerError)
                    .await?
                    .with_plain_text_body("Failed to save WiFi configuration")
                    .await
            }
        }
    }

    async fn api_date_time<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        _request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        log::debug!("Serving date_time request");

        let mut rtc = self.context.rtc().lock().await;
        let datetime = rtc.datetime().await.map_err(|e| {
            log::error!("RTC datetime read error: {}", e);
            Error::ServerError
        })?;

        let mut date_time_str = heapless::String::<64>::new();

        //ISO 8601 format could be used as well
        //"1995-12-17T03:24:00Z"
        core::fmt::write(&mut date_time_str, format_args!("{}", datetime)).map_err(|_| Error::ServerError)?;
        HttpResponseBuilder::new(http_socket)
            .with_status(StatusCode::Ok)
            .await?
            .with_plain_text_body(&date_time_str)
            .await
    }

    async fn api_set_date_time<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        log::debug!("Serving set_date_time request");
        let date_time_str = core::str::from_utf8(request.body).map_err(|_| {
            log::error!("Invalid UTF-8 in request body");
            Error::ServerError
        })?;
        let date_time = NaiveDateTime::from_str(date_time_str).map_err(|_| {
            log::error!("Invalid date time format: {}", date_time_str);
            Error::ServerError
        })?;

        let mut rtc = self.context.rtc().lock().await;
        rtc.set_datetime(&date_time).await.map_err(|e| {
            log::error!("RTC datetime set error: {}", e);
            Error::ServerError
        })?;

        HttpResponseBuilder::new(http_socket)
            .with_status(StatusCode::Ok)
            .await?
            .with_plain_text_body("Date and time updated")
            .await
    }

    async fn api_not_found<HttpSocket: HttpWriteSocket>(
        &mut self,
        _allocator: &mut PrefixArena<'_>,
        _request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        HttpResponseBuilder::new(http_socket)
            .with_status(StatusCode::NotFound)
            .await?
            .with_plain_text_body("Not Found")
            .await
    }

    async fn handle_rest_api<HttpSocket: HttpWriteSocket>(
        &mut self,
        allocator: &mut PrefixArena<'_>,
        request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
        api: &str,
    ) -> Result<(), Error> {
        match (request.method, api) {
            (HttpMethod::GET, "version") => self.api_version(allocator, request, http_socket).await,
            (HttpMethod::GET, "reboot") => self.api_reboot(allocator, request, http_socket).await,
            (HttpMethod::GET, "wifi_config") => self.api_wifi_config(allocator, request, http_socket).await,
            (HttpMethod::POST, "set_wifi_config") => self.api_set_wifi_config(allocator, request, http_socket).await,
            (HttpMethod::GET, "date_time") => self.api_date_time(allocator, request, http_socket).await,
            (HttpMethod::POST, "set_date_time") => self.api_set_date_time(allocator, request, http_socket).await,
            _ => self.api_not_found(allocator, request, http_socket).await,
        }
    }

    async fn handle_request_impl<HttpSocket: HttpWriteSocket>(
        &mut self,
        allocator: &mut PrefixArena<'_>,
        request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        if request.path == "/" {
            // Show main page
            log::debug!("Serving main configuration page");

            return HttpResponseBuilder::new(http_socket)
                .with_status(StatusCode::Ok)
                .await?
                .with_compressed_page(MAIN_CONFIGURATION_HTML_GZ)
                .await;
        }

        let Some(api) = request.path.strip_prefix("/api/") else {
            return HttpResponseBuilder::new(http_socket)
                .with_status(StatusCode::NotFound)
                .await?
                .with_plain_text_body("Not Found")
                .await;
        };

        if request.method == HttpMethod::OPTIONS {
            log::debug!("Serving {} preflight request", api);
            trace_headers(request);
            //TODO: Implement more strict header checking
            return HttpResponseBuilder::new(http_socket).preflight_response().await;
        }

        self.handle_rest_api(allocator, request, http_socket, api).await
    }

    async fn handle_websocket_connection_impl<'h>(
        &mut self,
        _request: &HttpRequest<'_>,
        web_socket: &mut WebSocket<'h, '_>,
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

impl<'a> HttpHandler for HttpWebAPIHandler<'a> {
    async fn handle_request<HttpSocket: HttpWriteSocket>(
        &mut self,
        allocator: &mut PrefixArena<'_>,
        request: &HttpRequest<'_>,
        http_socket: &mut HttpSocket,
    ) -> Result<(), Error> {
        self.context
            .shared_resources()
            .led_controller
            .set_animation(LED_1, LedAnimation::Decay(color::ORANGE, 300))
            .await;

        self.handle_request_impl(allocator, request, http_socket).await
    }

    async fn handle_websocket_connection<'h>(
        &mut self,
        request: &HttpRequest<'_>,
        web_socket: &mut WebSocket<'h, '_>,
    ) -> Result<(), ()> {
        self.context
            .shared_resources()
            .led_controller
            .set_animation(LED_1, LedAnimation::Decay(color::ORANGE, 300))
            .await;

        let res = self.handle_websocket_connection_impl(request, web_socket).await;

        res
    }
}

async fn send_serialized_type<T, WriteSocket: HttpWriteSocket>(
    allocator: &mut PrefixArena<'_>,
    http_socket: &mut WriteSocket,
    value: &T,
) -> Result<(), Error>
where
    T: serde::Serialize,
{
    let mut temp_buf = allocator.view();
    let value_buf = temp_buf.init_with(|uninitialized| {
        serde_json_core::to_slice(value, uninitialized).map_err(|e| {
            log::error!("Serialization error: {}", e);
            Error::ServerError
        })
    })?;

    HttpResponseBuilder::new(http_socket)
        .with_status(StatusCode::Ok)
        .await?
        .with_header("Content-Type", "application/json")
        .await?
        .with_body_from_slice(value_buf)
        .await
}

fn from_request<'de, T>(request: &HttpRequest<'de>) -> Result<T, nanofish::Error>
where
    T: serde::Deserialize<'de>,
{
    let (value, _) = serde_json_core::from_slice(request.body).map_err(|e| {
        log::error!("Deserialization error: {}", e);
        nanofish::Error::ServerError
    })?;

    Ok(value)
}
