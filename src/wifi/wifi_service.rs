use core::default;

use super::wifi_controller::*;
use crate::configuration::{NetworkSettings, WiFiApSettings, WiFiSettings};
use cyw43::NetDriver;
use defmt::*;
use embassy_executor::Spawner;
use embassy_net::{ConfigV4, DhcpConfig, Ipv4Address, Ipv4Cidr, Stack, StackResources};
use embassy_rp::clocks::RoscRng;
use embassy_sync::{blocking_mutex::raw::NoopRawMutex, mutex::Mutex};
use heapless::Vec;
use leasehund::{DHCPServerBuffers, DHCPServerSocket, DhcpServer, TransactionEvent};
use static_cell::StaticCell;

const NETWORK_RESOURCES_SIZE: usize = 20;
const JOIN_RETRY_COUNT: u8 = 5;

type WiFiServiceImplType = Mutex<NoopRawMutex, WifiServiceImpl<'static>>;

static NETWORK_RESOURCES: StaticCell<StackResources<NETWORK_RESOURCES_SIZE>> = StaticCell::new();
static WIFI_SERVICE_IMPL: StaticCell<WiFiServiceImplType> = StaticCell::new();

pub enum ActiveMode {
    Idle,
    Join,
    Ap,
}

#[derive(Clone, Copy, defmt::Format, Debug)]
pub enum JoiningStatus {
    JoiningAP,
    ObtainingIP,
    Ready,
    Failed,
}

#[derive(Clone, Copy, defmt::Format, Debug)]
pub enum ApStatus {
    StartingAP,
    WaitingForClient,
    Ready,
}

pub struct WiFiServiceBuilder {
    wifi_control: WiFiController<'static, IdleState>,
    wifi_network_driver: NetDriver<'static>,
}

impl WiFiServiceBuilder {
    pub fn new(
        wifi_control: WiFiController<'static, IdleState>,
        wifi_network_driver: NetDriver<'static>,
    ) -> Self {
        Self {
            wifi_control,
            wifi_network_driver,
        }
    }

    fn take_appart(self) -> (WiFiController<'static, IdleState>, NetDriver<'static>) {
        (self.wifi_control, self.wifi_network_driver)
    }

    #[must_use]
    pub fn build(self, spawner: Spawner) -> WifiService {
        let (wifi_control, wifi_network_driver) = self.take_appart();
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
        info!("Spawn network driver task");
        spawner.spawn(net_driver_task(runner)).unwrap();

        // Run service routine
        let service_impl = WIFI_SERVICE_IMPL.init(Mutex::new(WifiServiceImpl::new(
            wifi_control.into(),
            net_stack,
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

    /// Get the current active mode
    pub async fn active_mode(&self) -> ActiveMode {
        let service_impl = self.service_impl.lock().await;
        service_impl.active_mode()
    }

    /// Switch to idle mode
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

trait WiFiServiceImplementation<'a> {
    fn net_stack(&self) -> Stack<'a>;
    fn active_mode(&self) -> ActiveMode;

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
    net_stack: Stack<'a>,
    dhcp_server: Option<DhcpServer<2, 2>>,
}

impl<'a> WiFiServiceImplementation<'a> for WifiServiceImpl<'a> {
    fn net_stack(&self) -> Stack<'a> {
        self.net_stack
    }

    async fn idle(&mut self) {
        // Disable DHCP server in idle mode
        self.reset_dhcp_server();

        self.wifi_control
            .change_async(async |state| Self::idle_transition(state, self.net_stack).await)
            .await;
    }

    async fn join<H>(&mut self, wifi_settings: &WiFiSettings, mut join_status_handler: H)
    where
        H: AsyncFnMut(JoiningStatus) -> (),
    {
        // No DHCP server in client mode
        self.reset_dhcp_server();

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
        self.init_dhcp_server();

        wifi_state_handler(ApStatus::WaitingForClient).await;
        // Wait for a client to connect and get an IP address
        self.wait_for_dhcp_client().await.ok();
        wifi_state_handler(ApStatus::Ready).await;
    }

    fn active_mode(&self) -> ActiveMode {
        match &self.wifi_control {
            WiFiCtrlState::Idle(_) => ActiveMode::Idle,
            WiFiCtrlState::Joined(_) => ActiveMode::Join,
            WiFiCtrlState::Ap(_) => ActiveMode::Ap,
            WiFiCtrlState::Uninitialized => {
                defmt::unreachable!()
            }
        }
    }
}

impl<'a> WifiServiceImpl<'a> {
    fn new(wifi_control: WiFiCtrlState<'static>, net_stack: Stack<'a>) -> Self {
        Self {
            wifi_control,
            net_stack,
            dhcp_server: None,
        }
    }

    async fn idle_transition<'tr>(
        wifi_control_state: WiFiCtrlState<'tr>,
        net_stack: Stack<'tr>,
    ) -> WiFiCtrlState<'tr> {
        // Implement transition to Idle state
        info!("Transitioning to Idle state...");
        let mut controller = match wifi_control_state {
            WiFiCtrlState::Joined(controller) => controller.leave().await,
            WiFiCtrlState::Ap(controller) => controller.close_ap().await,
            WiFiCtrlState::Idle(idle) => idle,
            WiFiCtrlState::Uninitialized => {
                defmt::panic!("WiFi controller in uninitialized state, cannot transition to Idle");
            }
        };

        debug!("Waiting for link down...");
        net_stack.wait_link_down().await;
        // TODO: Check if this step is necessary
        debug!("Waiting for config down...");
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
        info!("Transitioning to Ap state...");

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

            debug!("Starting AP mode...");

            let password: heapless::String<64> =
                wifi_ap_settings.password.clone().unwrap_or_default();

            let mut ap_controller = if password.is_empty() {
                // Use open AP if password is empty
                controller
                    .start_ap_open(wifi_ap_settings.ssid.as_str(), wifi_ap_settings.channel)
                    .await
            } else {
                controller
                    .start_ap_wpa2(
                        wifi_ap_settings.ssid.as_str(),
                        password.as_str(),
                        wifi_ap_settings.channel,
                    )
                    .await
            };

            // TODO: notify process of AP mode start
            // debug!("Waiting for link up...");
            // net_stack.wait_link_up().await;
            debug!("Waiting for config up...");
            net_stack.wait_config_up().await;

            debug!("AP mode ready.");
            ap_controller.led(true).await;

            ap_controller.into()
        } else {
            // Should not reach here
            defmt::unreachable!()
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
        info!("Joining to WiFi ap state...");

        // TODO: Not quit sure if we need to go to idle first, but doing it for safety
        controller_state = Self::idle_transition(controller_state, net_stack).await;

        debug!("Attempting to join SSID: {}", wifi_settings.ssid.as_str());
        let mut join_options = JoinOptions::new(wifi_settings.password.as_bytes());
        join_options.auth = if wifi_settings.password.is_empty() {
            debug!("Using open authentication");
            JoinAuth::Open
        } else {
            debug!("Using WPA2 authentication");
            JoinAuth::Wpa2
        };

        for i in 0..JOIN_RETRY_COUNT {
            match controller_state {
                WiFiCtrlState::Idle(controller) => {
                    debug!("Attempt {}", i + 1);
                    controller_state = controller
                        .join(&wifi_settings.ssid, join_options.clone())
                        .await
                        .map_or_else(
                            |(idle, e)| {
                                error!("Join failed with status={}", e.status);
                                idle.into()
                            },
                            |joined| joined.into(),
                        )
                }

                WiFiCtrlState::Joined(mut controller) => {
                    debug!("Joined to a network.");
                    wifi_state_handler(JoiningStatus::ObtainingIP).await;

                    //Init DHCP client and wait for network read
                    let ip_config = if wifi_settings.use_static_ip_config {
                        if let Some(static_ip_config) = &wifi_settings.static_ip_config {
                            info!("Using static network settings: {}", static_ip_config);
                            ConfigV4::Static(static_ip_config.into())
                        } else {
                            error!(
                                "Static IP config selected but not configured, falling back to DHCP"
                            );
                            ConfigV4::Dhcp(DhcpConfig::default())
                        }
                    } else {
                        info!("Use DHCP provided network settings");
                        ConfigV4::Dhcp(DhcpConfig::default())
                    };

                    net_stack.set_config_v4(ip_config);

                    debug!("Waiting for link up...");
                    net_stack.wait_link_up().await;
                    debug!("Waiting for config up...");
                    net_stack.wait_config_up().await;
                    debug!("Connected to WiFi network.");

                    wifi_state_handler(JoiningStatus::Ready).await;

                    controller.led(true).await;
                    return controller.into();
                }

                _ => {
                    defmt::unreachable!()
                }
            }
        }

        wifi_state_handler(JoiningStatus::Failed).await;
        controller_state
    }

    fn reset_dhcp_server(&mut self) {
        self.dhcp_server = None;
    }

    fn init_dhcp_server(&mut self) {
        if let Some(config) = self.net_stack.config_v4() {
            let adr: Ipv4Cidr = config.address;
            let adr_oct = adr.address().octets();
            let start = Ipv4Address::new(adr_oct[0], adr_oct[1], adr_oct[2], adr_oct[3] + 122);
            let end = Ipv4Address::new(adr_oct[0], adr_oct[1], adr_oct[2], 255);
            let server: DhcpServer<2, 2> = DhcpServer::new(
                adr.address(),            // Server IP
                adr.netmask(),            // Subnet mask
                adr.address(),            // Gateway
                Ipv4Address::UNSPECIFIED, // DNS server
                start,                    // Pool start
                end,                      // Pool end
            );
            self.dhcp_server = Some(server);
        } else {
            error!("Cannot init DHCP server, no valid network config");
            self.reset_dhcp_server();
        }
    }

    async fn wait_for_dhcp_client(&mut self) -> Result<(Ipv4Address, [u8; 6]), ()> {
        let mut buffers = DHCPServerBuffers::new();
        let mut socket = DHCPServerSocket::new(self.net_stack, &mut buffers);

        let dhcp_server = self.dhcp_server.as_mut().ok_or(())?;

        loop {
            if dhcp_server.is_pool_full() {
                // In case there is no free IP addresses, we cannot lease any more.
                // Just stop the process.
                error!("No free ip-addresses for leasing");
                // Yeald to other tasks before returning
                embassy_futures::yield_now().await;
            }

            match dhcp_server.lease_one(&mut socket).await {
                Ok(TransactionEvent::Leased(ip, mac)) => {
                    info!("Leased IP: {} for MAC: {}", ip, mac);
                    // Wait a bit before returning to let the stack send the ACK packet
                    return Ok((ip, mac));
                }
                Err(e) => {
                    error!("DHCP server error: {:?}", e);
                    embassy_futures::yield_now().await;
                }
                _ => { /* Unsupported events, continue waiting */ }
            }
        }
    }
}

/* Tasks */
#[embassy_executor::task]
async fn net_driver_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
