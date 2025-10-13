use core::str::FromStr;
use defmt::*;

use cyw43::NetDriver;
use embassy_executor::Spawner;
use embassy_net::ConfigV4;
use embassy_net::DhcpConfig;
use embassy_net::Ipv4Address;
use embassy_net::Ipv4Cidr;
use embassy_net::Stack;
use embassy_net::StackResources;

use embassy_rp::clocks::RoscRng;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};
use embassy_time::Duration;
use embassy_time::Timer;
use heapless::Vec;
use leasehund::DHCPServerBuffers;
use leasehund::DHCPServerSocket;
use leasehund::TransactionEvent;
use static_cell::StaticCell;

use crate::flash_storage::*;
use crate::settings::Settings;
use crate::ui::*;
use crate::units::TimeExt as _;
use crate::vcp_sensors::*;
use crate::wifi::*;

// TODO: Move to separate module
// DHCP server
use core::net::Ipv4Addr;
use leasehund::DhcpServer;

pub const VCP_SENSORS_EVENT_QUEUE_SIZE: usize = 8;
const NETWORK_RESOURCES_SIZE: usize = 20;
const DEFAULT_AP_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
const DEFAULT_AP_SSID: &str = "LeadBarry";
const DEFAULT_AP_CHANNEL: u8 = 6;

static NETWORK_RESOURCES: StaticCell<StackResources<NETWORK_RESOURCES_SIZE>> = StaticCell::new();

pub type VcpControlType<'a> = VcpControl<'a, VCP_SENSORS_EVENT_QUEUE_SIZE>;
pub type UiControlType<'a> = UiControl<'a, ScCollection>;

pub async fn main_logic_controller(
    spawner: Spawner,
    vcp_control: &'static VcpControlType<'_>,
    ui_control: &'static UiControlType<'_>,
    wifi_control: IdleState<'_>,
    wifi_network_driver: NetDriver<'static>,
    shared_storage: &'static Mutex<CriticalSectionRawMutex, Storage<'static>>,
) -> ! {
    // try load settings from flash
    let mut settings = get_settings(shared_storage).await;

    // Generate random seed
    let mut rng = RoscRng;
    let seed = rng.next_u64();

    let config = embassy_net::Config::ipv4_static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(DEFAULT_AP_IP, 24),
        dns_servers: Vec::new(),
        gateway: Some(DEFAULT_AP_IP),
    });

    // Init network stack
    let stack_resources = NETWORK_RESOURCES.init(StackResources::new());
    let (stack, runner) = embassy_net::new(wifi_network_driver, config, stack_resources, seed);

    spawner.spawn(net_task(runner)).unwrap();
    let mut state = WiFiController::Idle(wifi_control);

    if !settings.wifi_ssid.is_empty() {
        // There is a wifi ssid configured, try to join
        state = join_wifi_network(state, &settings, ui_control, stack).await;
    }

    if matches!(state, WiFiController::Idle(_)) {
        // Still in idle so, switch to AP mode
        let WiFiController::Idle(controller) = state else {
            defmt::panic!("Unexpected state");
        };
        state = init_wifi_ap_network_and_wait_for_client(
            spawner,
            controller,
            &mut settings,
            ui_control,
            stack,
        )
        .await;

        // Here we ready to start web server for configuration
        // TODO: Implement web server
    }

    loop {
        // Main logic goes here
        Timer::after(Duration::from_secs(60)).await;
    }
}

async fn get_settings(shared_storage: &Mutex<CriticalSectionRawMutex, Storage<'_>>) -> Settings {
    let mut storage = shared_storage.lock().await;
    Settings::load_async(&mut storage)
        .await
        .unwrap_or_else(|e| {
            error!("Failed to load settings, using defaults ({:?})", e);
            let default_settings = Settings::default();
            if let Err(e) = default_settings.save(&mut storage) {
                error!("Failed to save default settings ({:?})", e);
            }
            default_settings
        })
}

async fn wait_for_network_ready(stack: &Stack<'_>) {
    loop {
        if stack.is_link_up() && stack.is_config_up() {
            // Additional check: try to get our own IP
            if let Some(ip) = stack.config_v4() {
                info!("Network stack ready: IpConfig {:?}", ip);
                break;
            }
        }
        Timer::after(Duration::from_millis(100)).await;
    }
}

async fn join_wifi_network<'a>(
    state: WiFiController<'a>,
    settings: &Settings,
    ui_control: &'static UiControlType<'_>,
    stack: Stack<'a>,
) -> WiFiController<'a> {
    // Shortcut for switching screens convenience
    let set_screen = |new_screen: ScCollection| async { ui_control.switch(new_screen).await };

    info!("Joining WiFi network: {}", settings.wifi_ssid);
    let mut state = state;
    for _ in 0..5 {
        state = match state {
            WiFiController::Idle(s) => {
                let wifi_status =
                    ScWifiStatsData::new(ScvState::Connecting, Some(settings.wifi_ssid.clone()));
                set_screen(ScWifiStats::new(wifi_status).into()).await;

                let mut join_options = JoinOptions::new(settings.wifi_password.as_bytes());
                join_options.auth = if settings.wifi_password.is_empty() {
                    JoinAuth::Open
                } else {
                    JoinAuth::Wpa2
                };

                match s.join(&settings.wifi_ssid, join_options).await {
                    Ok(s) => WiFiController::Joined(s),
                    Err((s, e)) => {
                        error!("Join failed with status={}", e.status);
                        WiFiController::Idle(s)
                    }
                }
            }

            WiFiController::Joined(_) => {
                break;
            }
            _ => {
                error!("Unexpected state");
                return state;
            }
        }
    }
    info!(
        "WiFi controller is in Joined to {}",
        settings.wifi_ssid.as_str()
    );
    let wifi_status = ScWifiStatsData::new(ScvState::Connected, Some(settings.wifi_ssid.clone()));
    set_screen(ScWifiStats::new(wifi_status).into()).await;
    Timer::after(1.s()).await;

    //Init DHCP client and wait for network ready
    set_screen(ScvIpStatus::new(Ipv4Address::UNSPECIFIED, ScvIpState::GettingIp, 0).into()).await;

    stack.set_config_v4(ConfigV4::Dhcp(DhcpConfig::default()));
    stack.wait_link_up().await;
    stack.wait_config_up().await;
    wait_for_network_ready(&stack).await;

    set_screen(ScvIpStatus::new(Ipv4Address::UNSPECIFIED, ScvIpState::GettingIp, 0).into()).await;
    let ip = stack.config_v4().map_or_else(
        || {
            error!("No IPv4 address acquired");
            Ipv4Address::UNSPECIFIED
        },
        |c| {
            info!("Acquired IPv4 address: {:?}", c.address);
            c.address.address()
        },
    );
    set_screen(ScvIpStatus::new(ip, ScvIpState::GettingIp, 0).into()).await;
    Timer::after(3.s()).await;

    state
}

async fn init_wifi_ap_network_and_wait_for_client<'a>(
    spawner: Spawner,
    wifi_controller: IdleState<'a>,
    settings: &mut Settings,
    ui_control: &'static UiControlType<'_>,
    stack: Stack<'static>,
) -> WiFiController<'a> {
    //SoftAP provisioning mode.

    // Shortcut for switching screens convenience
    let set_screen = |new_screen: ScCollection| async { ui_control.switch(new_screen).await };

    info!("Starting WiFi AP network: {}", DEFAULT_AP_SSID);

    // Generate random password
    let password = generate_random_password();

    let credentials = ScvCredentials {
        ssid: heapless::String::<32>::from_str(DEFAULT_AP_SSID).unwrap(),
        password: password.clone(),
    };

    // Set wifi ap screen with not ready state
    let wifi_ap_data = ScWifiApData::NotReady;
    set_screen(ScWifiAp::new(wifi_ap_data).into()).await;

    // Set wifi stack to the AP mode
    let wifi_controller = wifi_controller
        .start_ap_wpa2(DEFAULT_AP_SSID, password.as_str(), DEFAULT_AP_CHANNEL)
        .await;

    // Configure static IP for the AP
    stack.set_config_v4(ConfigV4::Static(embassy_net::StaticConfigV4 {
        address: Ipv4Cidr::new(DEFAULT_AP_IP, 24),
        dns_servers: Vec::new(),
        gateway: None,
    }));

    let wifi_ap_data = ScWifiApData::ConfigUp;
    set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
    stack.wait_config_up().await;

    let wifi_ap_data = ScWifiApData::WaitingForClient(credentials);
    set_screen(ScWifiAp::new(wifi_ap_data).into()).await;

    if let Some(config) = stack.config_v4() {
        info!("AP Configured with IP: {:?}", config.address);

        let adr: Ipv4Cidr = config.address;
        let adr_oct = adr.address().octets();
        let start = Ipv4Addr::new(adr_oct[0], adr_oct[1], adr_oct[2], adr_oct[3] + 122);
        let end = Ipv4Addr::new(adr_oct[0], adr_oct[1], adr_oct[2], 255);

        let mut server: DhcpServer<2, 2> = DhcpServer::new(
            adr.address(),             // Server IP
            adr.netmask(),             // Subnet mask
            adr.address(),             // Gateway
            Ipv4Addr::new(8, 8, 8, 8), // DNS server
            start,                     // Pool start
            end,                       // Pool end
        );

        let (ip, mac) = loop {
            let Ok((ip, mac)) = wait_for_dhcp_client(&mut server, stack).await else {
                Timer::after(1000.ms()).await;
                continue;
            };

            break (ip, mac);
        };

        let client_info = ScvClientInfo { ip, mac: Some(mac) };
        let wifi_ap_data = ScWifiApData::Connected(client_info);
        set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
    }

    // We have done

    WiFiController::Ap(wifi_controller)
}

/* Helper Functions */
// Generate random password
fn generate_random_password() -> heapless::String<64> {
    let mut rng = RoscRng;
    let mut pwd = heapless::String::<64>::new();
    for _ in 0..8 {
        let idx = (rng.next_u32() % 62) as u8;
        let c = if idx < 10 {
            (b'0' + idx) as char
        } else if idx < 36 {
            (b'a' + idx - 10) as char
        } else {
            (b'A' + idx - 36) as char
        };
        pwd.push(c).ok();
    }
    pwd
}

async fn wait_for_dhcp_client(
    server: &mut DhcpServer<2, 2>,
    stack: Stack<'static>,
) -> Result<(Ipv4Addr, [u8; 6]), ()> {
    let mut buffers = DHCPServerBuffers::new();
    let mut socket = DHCPServerSocket::new(stack, &mut buffers);
    loop {
        if server.is_pool_full() {
            // In case there is no free IP addresses, we cannot lease any more.
            // Just stop the process.
            error!("No free ip-addresses for leasing");
            return Err(());
        }

        match server.lease_one(&mut socket).await {
            Ok(TransactionEvent::Leased(ip, mac)) => {
                info!("Leased IP: {} for MAC: {}", ip, mac);
                // Wait a bit before returning to let the stack send the ACK packet
                return Ok((ip, mac));
            }
            Err(e) => {
                error!("DHCP server error: {:?}", e);
                return Err(());
            }
            _ => { /* Unsupported events, continue waiting */ }
        }
    }
}

/* Tasks */
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
