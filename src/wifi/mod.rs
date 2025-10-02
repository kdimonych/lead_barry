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
use embassy_net::{Config, Stack, StackResources};

use cyw43::JoinOptions;
use cyw43_firmware::{CYW43_43439A0, CYW43_43439A0_CLM};
use cyw43_pio::{DEFAULT_CLOCK_DIVIDER, PioSpi};
use defmt::*;
use static_cell::StaticCell;

use heapless::{String as HString, Vec as HVec};

// HTTP server
use nanofish::{
    DefaultHttpServer, HttpHandler, HttpRequest, HttpResponse, ResponseBody, StatusCode,
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
        .set_power_management(cyw43::PowerManagementMode::PowerSave)
        .await;

    let config = Config::dhcpv4(Default::default());
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
    static RESOURCES: StaticCell<StackResources<5>> = StaticCell::new();
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

    // And now we can use it!
    info!("Stack is up!");

    // This runs forever, handling requests
    run_server(stack).await;
}

// Create a simple request handler
struct MyHandler;

impl HttpHandler for MyHandler {
    async fn handle_request(
        &mut self,
        request: &HttpRequest<'_>,
    ) -> Result<HttpResponse<'_>, nanofish::Error> {
        match request.path {
            "/" => Ok(HttpResponse {
                status_code: StatusCode::Ok,
                headers: HVec::new(),
                body: ResponseBody::Text("<h1>Hello World!</h1>"),
            }),
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
    let mut server = DefaultHttpServer::new(80); // Listen on port 80
    let handler = MyHandler;

    // This runs forever, handling requests
    server.serve(stack, handler).await;
}
