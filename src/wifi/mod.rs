use embassy_net::tcp::{State, TcpSocket};
// Platform dependent crates
use embassy_rp::clocks::RoscRng;
// use embassy_rp::dma::Channel as DmaChannel;
use embassy_rp::gpio::{Level, Output, Pin};
use embassy_rp::peripherals::{DMA_CH0, PIN_23, PIN_24, PIN_25, PIN_29, PIO0};
use embassy_rp::pio::{Instance, InterruptHandler, Irq, Pio, StateMachine};
use embassy_rp::{Peri, bind_interrupts};

use embassy_executor::{Executor, Spawner};
use embassy_net::dns::DnsSocket;
use embassy_net::tcp::client::{TcpClient, TcpClientState};
use embassy_net::{Config, DhcpConfig, Stack, StackResources};

use cyw43::JoinOptions;
use cyw43_firmware::{CYW43_43439A0, CYW43_43439A0_CLM};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use embassy_time::{Duration, Timer};
use static_cell::StaticCell;

use heapless::{String as HString, Vec as HVec};

// HTTP server
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpRequest, HttpResponse, ResponseBody, SmallHttpServer,
    StatusCode,
};

bind_interrupts!(struct Irqs {
    PIO0_IRQ_0 => InterruptHandler<PIO0>;
});

static STATE: StaticCell<cyw43::State> = StaticCell::new();

pub struct WiFiSubsystemConfig {
    pub pwr_pin: Peri<'static, PIN_23>, // Power pin, pin 23
    pub cs_pin: Peri<'static, PIN_25>,  // Chip select pin, pin 25
    pub dio_pin: Peri<'static, PIN_24>, // Data In/Out pin, pin 24
    pub clk_pin: Peri<'static, PIN_29>, // Clock pin, pin 29
    pub pio: Peri<'static, PIO0>,       // PIO instance
    pub dma_ch: Peri<'static, DMA_CH0>, // DMA channel
    pub wifi_network: HString<32>,
    pub wifi_password: HString<63>,
}

#[embassy_executor::task]
async fn cyw43_task(
    runner: cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO0, 0, DMA_CH0>>,
) -> ! {
    runner.run().await
}

#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}

async fn wait_for_network_ready(stack: &Stack<'_>) {
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            // Additional check: try to get our own IP
            if let Some(_ip) = stack.config_v4() {
                info!("Network stack ready");
                break;
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

#[embassy_executor::task]
pub async fn wifi_task(spawner: Spawner, wifi_cfg: WiFiSubsystemConfig) -> ! {
    let fw = CYW43_43439A0; // Firmware binary included in the cyw43_firmware crate;
    let clm = CYW43_43439A0_CLM; // CLM binary included in the cyw43_firmware crate;
    // To make flashing faster for development, you may want to flash the firmwares independently
    // at hardcoded addresses, instead of baking them into the program with `include_bytes!`:
    //     probe-rs download 43439A0.bin --binary-format bin --chip RP2040 --base-address 0x10100000
    //     probe-rs download 43439A0_clm.bin --binary-format bin --chip RP2040 --base-address 0x10140000
    // let fw = unsafe { core::slice::from_raw_parts(0x10100000 as *const u8, 230321) };
    // let clm = unsafe { core::slice::from_raw_parts(0x10140000 as *const u8, 4752) };

    let pwr = Output::new(wifi_cfg.pwr_pin, Level::Low);
    let cs = Output::new(wifi_cfg.cs_pin, Level::High);
    let mut pio = Pio::new(wifi_cfg.pio, Irqs);

    let spi = PioSpi::new(
        &mut pio.common,
        pio.sm0,
        DEFAULT_CLOCK_DIVIDER,
        pio.irq0,
        cs,
        wifi_cfg.dio_pin,
        wifi_cfg.clk_pin,
        wifi_cfg.dma_ch,
    );

    let state = STATE.init(cyw43::State::new());
    let (net_device, mut control, runner) = cyw43::new(state, pwr, spi, fw).await;
    // Spawn the cyw43 (wifi) task
    spawner.spawn(cyw43_task(runner)).unwrap();

    control.init(clm).await;
    control
        .set_power_management(cyw43::PowerManagementMode::Performance)
        .await;
    control.gpio_set(0, true).await; // Turn on the LED

    let mut dhcp_config = DhcpConfig::default();
    // Accept most DHCP settings but ignore DNS
    dhcp_config.ignore_naks = false;

    let config = Config::dhcpv4(dhcp_config);
    // Use static IP configuration instead of DHCP
    //let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
    //    address: Ipv4Cidr::new(Ipv4Address::new(192, 168, 69, 2), 24),
    //    dns_servers: Vec::new(),
    //    gateway: Some(Ipv4Address::new(192, 168, 69, 1)),
    //});

    // Generate random seed
    let mut rng = RoscRng;
    let seed = rng.next_u64();

    // Init network stack
    static RESOURCES: StaticCell<StackResources<20>> = StaticCell::new();
    let (stack, runner) = embassy_net::new(
        net_device,
        config,
        RESOURCES.init(StackResources::new()),
        seed,
    );

    // Spawn the network task
    spawner.spawn(net_task(runner)).unwrap();

    while let Err(err) = control
        .join(
            wifi_cfg.wifi_network.as_str(),
            JoinOptions::new(wifi_cfg.wifi_password.as_bytes()),
        )
        .await
    {
        info!("join failed with status={}", err.status);
    }

    info!("waiting for link...");
    stack.wait_link_up().await;

    info!("waiting for DHCP...");
    stack.wait_config_up().await;

    info!("waiting for network ready...");
    wait_for_network_ready(&stack).await;
    // And now we can use it!
    info!("Stack is up!");

    let mut rx_buffer = [0; 1024];
    let mut tx_buffer = [0; 1024];
    let mut buf = heapless::String::<1024>::new();

    let mut req_counter = 0;

    let mut socket = TcpSocket::new(stack, &mut rx_buffer, &mut tx_buffer);
    socket.set_timeout(Some(Duration::from_secs(100)));
    socket.set_keep_alive(Some(Duration::from_secs(5)));

    loop {
        info!("Accepting connection...");
        if let Err(e) = socket.accept(80).await {
            defmt::warn!("Accept error: {:?}", e);
            Timer::after(Duration::from_millis(100)).await;
            continue;
        }

        if let Some(endpoint) = socket.remote_endpoint() {
            info!("Connection from {:?}", endpoint);
        }

        unsafe {
            // Just reuse the internal buffer of the buf string as a buffer to store the request
            let internal_vec = buf.as_mut_vec();
            internal_vec.clear(); // Clear any existing data
            internal_vec.resize(1024, 0).ok();
            let bytes = socket.read(internal_vec).await.unwrap();
            internal_vec.truncate(bytes);
            info!("Received: {:?}", buf.as_str());
        };

        buf.clear();

        core::fmt::write(
            &mut buf,
            format_args!(
                "HTTP/1.1 200 OK\r\nContent-Type: text/plain\r\n\r\nHello World! {}",
                req_counter
            ),
        )
        .ok();
        req_counter += 1;
        info!("Waiting the socket to get ready to write response...");
        socket.wait_write_ready().await;
        info!("Socket ready to write.");
        let w = socket.write(buf.as_bytes()).await.unwrap();
        info!("Wrote {} bytes. Buffer: {:?}", w, buf.as_str());
        socket.flush().await.unwrap();
        info!("Flushed socket.");
        let mut old_state = socket.state();
        info!("State before close {:?}", old_state);
        socket.close();
        old_state = socket.state();
        info!("State after close {:?}", old_state);
        socket.flush().await.unwrap();
        old_state = socket.state();
        info!("State after flush {:?}", old_state);
        // Wait until the socket is fully closed
        while socket.state() != State::FinWait2
            && socket.state() != State::Closed
            && socket.state() != State::TimeWait
        {
            let new_state = socket.state();
            if new_state != old_state {
                info!("Socket state changed to {:?}", new_state);
                old_state = new_state;
            }
            Timer::after(Duration::from_millis(40)).await;
        }
        socket.abort();
        info!("Aborted socket.");
        old_state = socket.state();
        info!("State after abort {:?}", old_state);
        while socket.state() != State::Closed {
            let new_state = socket.state();
            if new_state != old_state {
                info!("Socket state changed to {:?}", new_state);
                old_state = new_state;
            }
            Timer::after(Duration::from_millis(40)).await;
        }
    }

    // This runs forever, handling requests
    //run_server(stack).await;
}

// Create a simple request handler
struct MyHandler;

impl HttpHandler for MyHandler {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        match request.path {
            "/" => {
                debug!("Received request for path: {}", request.path);
                Ok(HttpResponse {
                    status_code: StatusCode::Ok,
                    headers: HVec::new(),
                    body: ResponseBody::Text("<h1>Hello World!</h1>"),
                })
            }
            "/api/status" => Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: HVec::new(),
                body: ResponseBody::Text("{\"status\":\"ok\"}"),
            }),
            _ => Ok(HttpResponse {
                status_code: StatusCode::NotFound,
                headers: HVec::new(),
                body: ResponseBody::Text("Not Found"),
            }),
        }
    }
}

async fn run_server(stack: Stack<'_>) -> ! {
    let mut server = SmallHttpServer::new(80); // Listen on port 80
    let handler = MyHandler;

    // This runs forever, handling requests
    server.serve(stack, handler).await
}
