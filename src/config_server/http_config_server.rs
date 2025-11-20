// use cyw43::NetDriver;
use crate::configuration::{ConfigurationStorage, Settings};
use crate::{reset, units::TimeExt as _};
use defmt::info;
use embassy_executor::Spawner;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{HttpHandler, HttpRequest, HttpResponse, HttpServer, ResponseBody, StatusCode};
use serde::{Deserialize, Serialize};

const RX_SIZE: usize = 2048;
const TX_SIZE: usize = 2048;
const REQ_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 8192;

//const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
const MAIN_CONFIGURATION_HTML_GZ: &[u8] = include_bytes!("./web/main_configuration.html.gz");
//include_packed = "0.1.5"
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
            context: HttpServerContext {
                spawner,
                configuration_storage,
            },
            http_server,
        }
    }

    pub async fn run(&mut self, stack: Stack<'_>) -> ! {
        self.http_server
            .serve(stack, HttpConfigHandler::new(&self.context))
            .await;
    }
}

struct HttpServerContext {
    spawner: Spawner,
    configuration_storage: &'static ConfigurationStorage<'static>,
}

// Create a simple request handler
struct HttpConfigHandler<'a> {
    context: &'a HttpServerContext,
    content_length: heapless::String<32>,
}

impl<'a> HttpConfigHandler<'a> {
    fn new(context: &'a HttpServerContext) -> Self {
        Self {
            context,
            content_length: heapless::String::<32>::new(),
        }
    }

    fn prepare_main_page_response(&mut self) -> Result<HttpResponse<'_>, nanofish::Error> {
        let mut response = HttpResponse {
            status_code: StatusCode::Ok,
            headers: Vec::new(),
            body: ResponseBody::Binary(MAIN_CONFIGURATION_HTML_GZ),
        };

        response
            .headers
            .push(nanofish::HttpHeader::new("Content-Encoding", "gzip"))
            .map_err(|_| nanofish::Error::InvalidStatusCode)?;

        core::fmt::write(
            &mut self.content_length,
            format_args!("{}", MAIN_CONFIGURATION_HTML_GZ.len()),
        )
        .map_err(|_| nanofish::Error::InvalidStatusCode)?;

        response
            .headers
            .push(nanofish::HttpHeader::new(
                "Content-Length",
                self.content_length.as_str(),
            ))
            .map_err(|_| nanofish::Error::InvalidStatusCode)?;

        response
            .headers
            .push(nanofish::HttpHeader::new(
                "Content-Type",
                "text/html; charset=utf-8",
            ))
            .map_err(|_| nanofish::Error::InvalidStatusCode)?;

        info!(
            "Send main page. Compressed size: {}",
            MAIN_CONFIGURATION_HTML_GZ.len()
        );

        Ok(response)
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

            return self.prepare_main_page_response();
        }

        let Some(api) = request.path.strip_prefix("/api/") else {
            return Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            });
        };

        match api {
            "status" => Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            }),
            // "get_config" => {
            //     let settings = self
            //         .context
            //         .configuration_storage
            //         .get_settings()
            //         .await
            //         .clone();
            //     let config_json = serde_json::to_string(&settings).map_err(|e| {
            //         defmt::error!("Failed to serialize settings: {}", e);
            //         nanofish::Error::InternalServerError
            //     })?;
            //     Ok(HttpResponse {
            //         status_code: StatusCode::Ok,
            //         headers: Vec::new(),
            //         body: ResponseBody::Text(&config_json),
            //     })
            // }
            // "save_config" => {
            //     let settings: Settings =
            //         serde_json::from_str(request.body.as_str()).map_err(|e| {
            //             defmt::error!("Failed to deserialize JSON: {}", e);
            //             nanofish::Error::InternalServerError
            //         })?;

            //     // Here you would parse and save the configuration from the request body
            //     self.context
            //         .configuration_storage
            //         .set_settings(Settings::new())
            //         .await;
            //     self.context
            //         .configuration_storage
            //         .save()
            //         .await
            //         .map_err(|e| {
            //             defmt::error!("Failed to save configuration: {}", e);
            //             nanofish::Error::InternalServerError
            //         })?;
            //     defmt::info!("Configuration saved successfully");
            //     Ok(HttpResponse {
            //         status_code: StatusCode::Ok,
            //         headers: Vec::new(),
            //         body: ResponseBody::Text("{\"result\":\"config saved\"}"),
            //     })
            // }
            "reset" => {
                reset::deferred_system_reset(self.context.spawner, 1.s());
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
