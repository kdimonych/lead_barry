// use cyw43::NetDriver;
use crate::{reset, units::TimeExt as _};
use embassy_executor::Spawner;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{HttpHandler, HttpRequest, HttpResponse, HttpServer, ResponseBody, StatusCode};

const RX_SIZE: usize = 2048;
const TX_SIZE: usize = 2048;
const REQ_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 8192;

pub struct HttpConfigServer {
    context: HttpServerContext,
    http_server: HttpServer<RX_SIZE, TX_SIZE, REQ_SIZE, MAX_RESPONSE_SIZE>,
}

impl HttpConfigServer {
    pub fn new(spawner: Spawner) -> Self {
        let http_server = HttpServer::new(80);
        Self {
            context: HttpServerContext { spawner },
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
            Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("<h1>Hello World!</h1>"),
            })
        } else if let Some(api) = request.path.strip_prefix("/api/") {
            match api {
                "status" => Ok(HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text("{\"status\":\"ok\"}"),
                }),
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
        } else {
            Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            })
        }
    }
}
