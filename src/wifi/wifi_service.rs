use super::wifi_controller::*;
use crate::{
    configuration::{WiFiApSettings, WiFiSettings},
    wifi::{WiFiConfig, dhcp_server::DhcpEvent},
};

use super::dhcp_server::{DhcpServer, DhcpServerConfig, DhcpServerState};
use cyw43_pio::PioSpi;
use defmt_or_log as log;
use embassy_executor::Spawner;
use embassy_net::{ConfigV4, DhcpConfig, Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_rp::{
    clocks::RoscRng, gpio::Output, interrupt::typelevel::Binding, pio::InterruptHandler,
};
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use heapless::Vec;
use static_cell::StaticCell;

const NETWORK_RESOURCES_SIZE: usize = 20;
const JOIN_RETRY_COUNT: u8 = 5;

type WiFiServiceImplType = Mutex<NoopRawMutex, WifiServiceImpl<'static>>;

static NETWORK_RESOURCES: StaticCell<StackResources<NETWORK_RESOURCES_SIZE>> = StaticCell::new();
static WIFI_SERVICE_IMPL: StaticCell<WiFiServiceImplType> = StaticCell::new();
static WIFI_STATIC_DATA: StaticCell<WiFiStaticData> = StaticCell::new();
static DHCP_SERVER_STATE: StaticCell<DhcpServerState> = StaticCell::new();

#[derive(Clone, Copy, Debug)]
#[defmt_or_log::derive_format_or_debug]
pub enum JoiningStatus {
    JoiningAP,
    Dhcp,
    Ready,
    Failed,
}

#[derive(Clone, Copy, Debug)]
#[defmt_or_log::derive_format_or_debug]
pub enum ApStatus {
    StartingAP,
    WaitingForClient,
    Ready((Ipv4Address, [u8; 6])),
}

pub struct WiFiServiceBuilder<PIO, DMA>
where
    // Bounds from impl:
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    wifi_driver_builder: WiFiDriverBuilder<WiFiBuilderCreated<PIO, DMA>>,
}

impl<PIO, DMA> WiFiServiceBuilder<PIO, DMA>
where
    // Bounds from impl:
    DMA: embassy_rp::dma::Channel + 'static,
    PIO: embassy_rp::pio::Instance + 'static,
{
    pub fn new(
        wifi_cfg: WiFiConfig<PIO, DMA>,
        irq: impl Binding<PIO::Interrupt, InterruptHandler<PIO>>,
    ) -> Self {
        Self {
            wifi_driver_builder: WiFiDriverBuilder::new(wifi_cfg, irq),
        }
    }

    fn take_appart(self) -> WiFiDriverBuilder<WiFiBuilderCreated<PIO, DMA>> {
        self.wifi_driver_builder
    }

    #[must_use]
    pub async fn build<SpawnTokenBuilder, S>(
        self,
        spawner: Spawner,
        wifi_runner_task: SpawnTokenBuilder,
    ) -> WifiService
    where
        SpawnTokenBuilder: Fn(
            cyw43::Runner<'static, Output<'static>, PioSpi<'static, PIO, 0, DMA>>,
        ) -> ::embassy_executor::SpawnToken<S>,
    {
        let wifi_driver_builder = self.take_appart();

        //Initialize wifi controller
        log::info!("Create wifi controller");
        let wifi_static_data = WIFI_STATIC_DATA.init(WiFiStaticData::new());
        let (wifi_controller, wifi_network_driver) = wifi_driver_builder
            .build(wifi_static_data, spawner, wifi_runner_task)
            .await;

        //let (wifi_control, wifi_network_driver) = self.take_appart();
        let mut rng = RoscRng;
        let seed = rng.next_u64();

        let stack_resources: &'static mut StackResources<NETWORK_RESOURCES_SIZE> =
            NETWORK_RESOURCES.init(StackResources::new());
        let (net_stack, runner) = embassy_net::new(
            wifi_network_driver,
            embassy_net::Config::dhcpv4(Default::default()),
            stack_resources,
            seed,
        );

        // Spawn network driver task
        log::info!("Spawn network driver task");
        spawner.spawn(net_driver_task(runner)).unwrap();

        // Initialize DHCP server state
        let dhcp_server_state = DHCP_SERVER_STATE.init(DhcpServerState::new());
        let dhcp_server = DhcpServer::new(dhcp_server_state).await;

        // Run service routine
        let service_impl = WIFI_SERVICE_IMPL.init(Mutex::new(WifiServiceImpl::new(
            wifi_controller.into(),
            net_stack,
            dhcp_server,
            spawner,
        )));

        WifiService { service_impl }
    }
}

/// WiFi Service
/// - Manages WiFi connection and access point modes
/// - Provides network stack access
/// - Handles DHCP server in access point mode
/// - Allows switching between idle, join, and access point modes
/// - Not thread-safe, should be used from a single task context
/// # Example
/// ```rust,no_run
/// use embassy_executor::Spawner;
/// use embassy_net::Stack;
/// use cyw43::NetDriver;
/// use crate::wifi::{WiFiServiceBuilder, WiFiController, IdleState,
///     NetworkSettings, JoiningStatus};
/// async fn example(spawner: Spawner, wifi_control: WiFiController<'static, IdleState>,
///     wifi_network_driver: NetDriver<'static>, network_settings: &NetworkSettings) {
/// let wifi_service = WiFiServiceBuilder::new(wifi_control, wifi_network_driver)
///     .build(spawner);
/// let net_stack: Stack<'static> = wifi_service.net_stack().await;
/// wifi_service.join(network_settings, async |status| {
///     match status {
///         JoiningStatus::JoiningAP => {
///             // Handle joining status
///         }
///         JoiningStatus::ObtainingIP => {
///             // Handle obtaining IP status
///         }
///         JoiningStatus::Ready => {
///             // Handle ready status
///         }
///         JoiningStatus::Failed => {
///             // Handle failed status
///         }
///     }
/// }).await;
/// # }
/// ```
pub struct WifiService {
    service_impl: &'static WiFiServiceImplType,
}

impl WifiService {
    /// Get a reference to the network stack
    pub async fn net_stack(&self) -> Stack<'static> {
        let service_impl = self.service_impl.lock().await;
        service_impl.net_stack()
    }

    /// Switch to idle mode
    #[allow(dead_code)]
    pub async fn idle(&self) {
        let mut service_impl = self.service_impl.lock().await;
        service_impl.idle().await;
    }

    /// Switch to join mode (connect to WiFi)
    pub async fn join<H>(&self, wifi_settings: &WiFiSettings, join_status_handler: H)
    where
        H: AsyncFnMut(JoiningStatus) -> (),
    {
        let mut service_impl = self.service_impl.lock().await;
        service_impl.join(wifi_settings, join_status_handler).await;
    }

    /// Switch to access point mode
    pub async fn start_ap<H>(&self, wifi_ap_settings: &WiFiApSettings, wifi_state_handler: H)
    where
        H: AsyncFnMut(ApStatus) -> (),
    {
        let mut service_impl = self.service_impl.lock().await;
        service_impl
            .start_ap(wifi_ap_settings, wifi_state_handler)
            .await;
    }
}

#[allow(dead_code)]
trait WiFiServiceImplementation<'a> {
    fn net_stack(&self) -> Stack<'a>;

    async fn idle(&mut self);
    async fn join<H>(&mut self, wifi_settings: &WiFiSettings, join_status_handler: H)
    where
        H: AsyncFnMut(JoiningStatus) -> ();
    async fn start_ap<H>(&mut self, wifi_ap_settings: &WiFiApSettings, wifi_state_handler: H)
    where
        H: AsyncFnMut(ApStatus) -> ();
}

struct WifiServiceImpl<'a> {
    wifi_control: WiFiCtrlState<'a>,
    net_stack: Stack<'static>,
    dhcp_server: DhcpServer,
    spawner: Spawner,
}

impl<'a> WiFiServiceImplementation<'a> for WifiServiceImpl<'a> {
    fn net_stack(&self) -> Stack<'a> {
        self.net_stack
    }

    async fn idle(&mut self) {
        // Disable DHCP server in idle mode
        self.reset_dhcp_server().await;

        self.wifi_control
            .change_async(async |state| Self::idle_transition(state, self.net_stack).await)
            .await;
    }

    async fn join<H>(&mut self, wifi_settings: &WiFiSettings, mut join_status_handler: H)
    where
        H: AsyncFnMut(JoiningStatus) -> (),
    {
        // No DHCP server in client mode
        self.reset_dhcp_server().await;

        join_status_handler(JoiningStatus::JoiningAP).await;

        self.wifi_control
            .change_async(async |state| {
                Self::join_transition(state, self.net_stack, join_status_handler, wifi_settings)
                    .await
            })
            .await;
    }

    async fn start_ap<H>(&mut self, wifi_ap_settings: &WiFiApSettings, mut wifi_state_handler: H)
    where
        H: AsyncFnMut(ApStatus) -> (),
    {
        wifi_state_handler(ApStatus::StartingAP).await;
        self.wifi_control
            .change_async(async |state| {
                Self::ap_transition(state, self.net_stack, wifi_ap_settings).await
            })
            .await;

        // Initialize DHCP server for AP mode
        self.init_dhcp_server().await;

        wifi_state_handler(ApStatus::WaitingForClient).await;
        log::trace!("Wait for client connected");
        // Wait for a client to connect and get an IP address
        let new_client = self.wait_for_dhcp_client().await.unwrap();
        log::trace!("Dhcp client has been connected.");
        wifi_state_handler(ApStatus::Ready(new_client)).await;
    }
}

impl<'a> WifiServiceImpl<'a> {
    fn new(
        wifi_control: WiFiCtrlState<'static>,
        net_stack: Stack<'static>,
        dhcp_server: DhcpServer,
        spawner: Spawner,
    ) -> Self {
        Self {
            wifi_control,
            net_stack,
            dhcp_server,
            spawner,
        }
    }

    async fn idle_transition<'tr>(
        wifi_control_state: WiFiCtrlState<'tr>,
        net_stack: Stack<'tr>,
    ) -> WiFiCtrlState<'tr> {
        // Implement transition to Idle state
        log::info!("Transitioning to Idle state...");
        let mut controller = match wifi_control_state {
            WiFiCtrlState::Joined(controller) => controller.leave().await,
            WiFiCtrlState::Ap(controller) => controller.close_ap().await,
            WiFiCtrlState::Idle(idle) => idle,
            WiFiCtrlState::Uninitialized => {
                log::panic!("WiFi controller in uninitialized state, cannot transition to Idle");
            }
        };

        log::debug!("Waiting for link down...");
        net_stack.wait_link_down().await;
        // TODO: Check if this step is necessary
        log::debug!("Waiting for config down...");
        net_stack.wait_config_down().await;

        // Ensure LED is off in idle state
        controller.led(false).await;

        controller.into()
    }

    async fn ap_transition<'tr>(
        mut controller_state: WiFiCtrlState<'tr>,
        net_stack: Stack<'tr>,
        wifi_ap_settings: &WiFiApSettings,
    ) -> WiFiCtrlState<'tr> {
        // Implement transition to Idle state
        log::info!("Transitioning to Ap state...");

        // TODO: Not quit sure if we need to go to idle first, but doing it for safety
        controller_state = Self::idle_transition(controller_state, net_stack).await;

        if let WiFiCtrlState::Idle(controller) = controller_state {
            // Set static IP config for AP mode
            let config = embassy_net::StaticConfigV4 {
                address: Ipv4Cidr::new(wifi_ap_settings.ip.into(), wifi_ap_settings.prefix_len),
                dns_servers: Vec::new(),
                gateway: None,
            };
            net_stack.set_config_v4(ConfigV4::Static(config));

            log::debug!("Starting AP mode...");

            let password: heapless::String<64> =
                wifi_ap_settings.password.clone().unwrap_or_default();

            let mut ap_controller = if password.is_empty() {
                log::debug!("Open AP mode...");
                // Use open AP if password is empty
                controller
                    .start_ap_open(wifi_ap_settings.ssid.as_str(), wifi_ap_settings.channel)
                    .await
            } else {
                log::debug!("WPA2 AP mode...");
                controller
                    .start_ap_wpa2(
                        wifi_ap_settings.ssid.as_str(),
                        password.as_str(),
                        wifi_ap_settings.channel,
                    )
                    .await
            };

            // TODO: notify process of AP mode start
            // log::debug!("Waiting for link up...");
            // net_stack.wait_link_up().await;
            log::debug!("Waiting for config up...");
            net_stack.wait_config_up().await;

            log::debug!("AP mode ready.");
            ap_controller.led(true).await;

            ap_controller.into()
        } else {
            // Should not reach here
            log::panic!("Unexpected state in AP transition");
        }
    }

    async fn join_transition<'tr, H>(
        mut controller_state: WiFiCtrlState<'tr>,
        net_stack: Stack<'tr>,
        mut wifi_state_handler: H,
        wifi_settings: &WiFiSettings,
    ) -> WiFiCtrlState<'tr>
    where
        H: AsyncFnMut(JoiningStatus) -> (),
    {
        log::info!("Joining to WiFi ap state...");

        // TODO: Not quit sure if we need to go to idle first, but doing it for safety
        controller_state = Self::idle_transition(controller_state, net_stack).await;

        log::debug!("Attempting to join SSID: {}", wifi_settings.ssid.as_str());

        let join_options = if let Some(psw) = &wifi_settings.password {
            let mut join_options = JoinOptions::new(psw.as_bytes());
            join_options.auth = if psw.is_empty() {
                log::debug!("Using open authentication as password is empty");
                JoinAuth::Open
            } else {
                log::debug!("Using WPA2/WPA3 authentication");
                JoinAuth::Wpa2Wpa3
            };
            join_options
        } else {
            log::debug!("Using open authentication");
            let mut join_options = JoinOptions::new(b"");
            join_options.auth = JoinAuth::Open;
            join_options
        };

        for i in 0..JOIN_RETRY_COUNT {
            match controller_state {
                WiFiCtrlState::Idle(controller) => {
                    log::debug!("Attempt {}", i + 1);
                    controller_state = controller
                        .join(&wifi_settings.ssid, join_options.clone())
                        .await
                        .map_or_else(
                            |(idle, e)| {
                                log::error!("Join failed with status={}", e.status);
                                idle.into()
                            },
                            |joined| joined.into(),
                        )
                }

                WiFiCtrlState::Joined(mut controller) => {
                    log::debug!("Joined to a network.");

                    //Init DHCP client and wait for network read
                    let ip_config = if wifi_settings.use_static_ip_config {
                        if let Some(static_ip_config) = &wifi_settings.static_ip_config {
                            log::info!("Using static network settings: {}", static_ip_config);
                            ConfigV4::Static(static_ip_config.into())
                        } else {
                            log::error!(
                                "Static IP config selected but not configured, falling back to DHCP"
                            );
                            ConfigV4::Dhcp(DhcpConfig::default())
                        }
                    } else {
                        log::info!("Use DHCP provided network settings");
                        ConfigV4::Dhcp(DhcpConfig::default())
                    };

                    if let &ConfigV4::Dhcp(_) = &ip_config {
                        wifi_state_handler(JoiningStatus::Dhcp).await;
                    }

                    net_stack.set_config_v4(ip_config);

                    log::debug!("Waiting for link up...");
                    net_stack.wait_link_up().await;
                    log::debug!("Waiting for config up...");
                    net_stack.wait_config_up().await;
                    log::debug!("Connected to WiFi network.");

                    wifi_state_handler(JoiningStatus::Ready).await;

                    controller.led(true).await;
                    return controller.into();
                }

                _ => {
                    log::unreachable!()
                }
            }
        }

        wifi_state_handler(JoiningStatus::Failed).await;
        controller_state
    }

    async fn reset_dhcp_server(&mut self) {
        self.dhcp_server.stop().await;
    }

    async fn init_dhcp_server(&mut self) {
        if let Some(config) = self.net_stack.config_v4() {
            let adr_oct = config.address.address().octets();
            let start = Ipv4Address::new(adr_oct[0], adr_oct[1], adr_oct[2], adr_oct[3] + 122);
            let end = Ipv4Address::new(adr_oct[0], adr_oct[1], adr_oct[2], 255);

            let dhcp_config = DhcpServerConfig::new(
                config.address.address(),
                config.address.netmask(),
                config.address.address(),
                start,
                end,
            );
            self.dhcp_server
                .start(self.spawner, self.net_stack, dhcp_config)
                .await;
        } else {
            log::error!("Cannot init DHCP server, no valid network config");
            self.reset_dhcp_server().await;
        }
    }

    async fn wait_for_dhcp_client(&mut self) -> Result<(Ipv4Address, [u8; 6]), ()> {
        loop {
            match self.dhcp_server.wait_event().await {
                DhcpEvent::Lease(ip, mac) => return Ok((ip, mac)),
                DhcpEvent::Release(_, _) => { /* Ignore release events */ }
            }
        }
    }
}

/* Tasks */
#[embassy_executor::task]
async fn net_driver_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
