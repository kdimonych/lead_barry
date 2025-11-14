// use cyw43::NetDriver;
use crate::{reset, units::TimeExt as _};
use embassy_executor::Spawner;
use embassy_net::Stack;
use heapless::Vec;
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpRequest, HttpResponse, HttpServer, ResponseBody, StatusCode,
};

const RX_SIZE: usize = 2048;
const TX_SIZE: usize = 2048;
const REQ_SIZE: usize = 1024;
const MAX_RESPONSE_SIZE: usize = 8192;

pub struct HttpConfigServer {
    http_server: HttpServer<RX_SIZE, TX_SIZE, REQ_SIZE, MAX_RESPONSE_SIZE>,
}

impl HttpConfigServer {
    pub fn new() -> Self {
        let http_server = HttpServer::new(80);
        Self { http_server }
    }

    pub async fn run(&mut self, spawner: Spawner, stack: Stack<'_>) -> ! {
        self.http_server
            .serve(stack, HttpConfigHandler { spawner })
            .await;
    }
}

// Create a simple request handler
struct HttpConfigHandler {
    spawner: Spawner,
}

impl HttpHandler for HttpConfigHandler {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        match request.path {
            "/reset" => {
                reset::deferred_system_reset(self.spawner, 1.s());
                // The reset function does not return, but we provide a response for completeness
                Ok(HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: Vec::new(),
                    body: ResponseBody::Text("System is resetting..."),
                })
            }
            "/" => Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("<h1>Hello World!</h1>"),
            }),
            "/api/status" => Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: Vec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            }),
            _ => Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: Vec::new(),
                body: ResponseBody::Text("Not Found"),
            }),
        }
    }
}
