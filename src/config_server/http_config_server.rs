// use cyw43::NetDriver;
use crate::configuration::{ConfigurationStorage, Settings};
use crate::{reset, units::TimeExt as _};
use embassy_executor::Spawner;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{HttpHandler, HttpRequest, HttpResponse, HttpServer, ResponseBody, StatusCode};
use serde::{Deserialize, Serialize};

const RX_SIZE: usize = 2048;
const TX_SIZE: usize = 2048;
const REQ_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 8192;

const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
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
            .serve(
                stack,
                HttpConfigHandler {
                    context: &self.context,
                },
            )
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
}

impl<'a> HttpHandler for HttpConfigHandler<'a> {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        if request.path == "/" {
            // Show main page
            return Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text(MAIN_CONFIGURATION_HTML),
            });
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
