use defmt::info;
use heapless::Vec;
use nanofish::{HttpRequest, HttpResponse, ResponseBody, StatusCode};

use super::http_server_context::HttpServerContext;
use super::temporal_handler::TemporalHttpHandler;

//const MAIN_CONFIGURATION_HTML: &str = include_str!("./web/main_configuration.html");
const MAIN_CONFIGURATION_HTML_GZ: &[u8] = include_bytes!("./web/main_configuration.html.gz");

pub struct MainPageHandler {
    content_length_str: heapless::String<32>,
}

impl MainPageHandler {
    pub const fn new() -> Self {
        Self {
            content_length_str: heapless::String::<32>::new(),
        }
    }
}

impl Default for MainPageHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl TemporalHttpHandler for MainPageHandler {
    async fn handle_request(
        &mut self,
        _request: &HttpRequest<'_>,
        _context: &'_ HttpServerContext,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
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
            &mut self.content_length_str,
            format_args!("{}", MAIN_CONFIGURATION_HTML_GZ.len()),
        )
        .map_err(|_| nanofish::Error::InvalidStatusCode)?;

        response
            .headers
            .push(nanofish::HttpHeader::new(
                "Content-Length",
                self.content_length_str.as_str(),
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
