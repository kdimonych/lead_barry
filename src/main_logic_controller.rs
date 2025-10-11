use core::error;
use core::str::FromStr;
use cortex_m::register::control;
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
use static_cell::StaticCell;

use crate::flash_storage::*;
use crate::settings::{Error as SettingsError, Settings};
use crate::ui::*;
use crate::units::TimeExt as _;
use crate::vcp_sensors::*;
use crate::wifi::*;
use common::any_string::AnyString;

pub const VCP_SENSORS_EVENT_QUEUE_SIZE: usize = 8;
const NETWORK_RESOURCES_SIZE: usize = 20;
const DEFAULT_AP_IP: Ipv4Address = Ipv4Address::new(192, 168, 1, 1);
const DEFAULT_AP_SSID: &str = "LeadBarry";

static NETWORK_RESOURCES: StaticCell<StackResources<NETWORK_RESOURCES_SIZE>> = StaticCell::new();

pub type VcpControlType<'a> = VcpControl<'a, VCP_SENSORS_EVENT_QUEUE_SIZE>;
pub type UiControlType<'a> = UiControl<'a, ScCollection>;

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

pub async fn main_logic_controller(
    spawner: Spawner,
    vcp_control: &'static VcpControlType<'_>,
    ui_control: &'static UiControlType<'_>,
    wifi_control: IdleState<'_>,
    wifi_network_driver: NetDriver<'static>,
    shared_storage: &'static Mutex<CriticalSectionRawMutex, Storage<'static>>,
) -> ! {
    // try load settings from flash
    let settings = get_settings(shared_storage).await;

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
        state = init_wifi_ap_network(state, &settings, ui_control, stack).await;
    }

    loop {
        // Main logic goes here
        Timer::after(Duration::from_secs(60)).await;
    }
}

async fn join_wifi_network<'a>(
    state: WiFiController<'a>,
    settings: &Settings,
    ui_control: &'static UiControlType<'_>,
    stack: Stack<'a>,
) -> WiFiController<'a> {
    info!("Joining WiFi network: {}", settings.wifi_ssid);
    let mut state = state;
    for try_count in 0..5 {
        state = match state {
            WiFiController::Idle(s) => {
                let wifi_status =
                    ScWifiStatsData::new(ScvState::Connecting, Some(settings.wifi_ssid.clone()));
                let wifi_status_screen = ScWifiStats::new(wifi_status);
                ui_control
                    .switch(ScCollection::WiFiStatus(wifi_status_screen))
                    .await;

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
    let wifi_status_screen = ScWifiStats::new(wifi_status);
    ui_control
        .switch(ScCollection::WiFiStatus(wifi_status_screen))
        .await;

    //Init DHCP client and wait for network ready
    ui_control
        .switch(ScCollection::IpStatus(ScvIpStatus::new(
            Ipv4Address::UNSPECIFIED,
            ScvIpState::GettingIp,
            0,
        )))
        .await;

    stack.set_config_v4(ConfigV4::Dhcp(DhcpConfig::default()));
    stack.wait_link_up().await;
    stack.wait_config_up().await;
    wait_for_network_ready(&stack).await;

    ui_control
        .switch(ScCollection::IpStatus(ScvIpStatus::new(
            Ipv4Address::UNSPECIFIED,
            ScvIpState::GettingIp,
            0,
        )))
        .await;
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
    ui_control
        .switch(ScCollection::IpStatus(ScvIpStatus::new(
            ip,
            ScvIpState::GettingIp,
            0,
        )))
        .await;

    Timer::after(3.s()).await;
    state
}

async fn init_wifi_ap_network<'a>(
    state: WiFiController<'a>,
    settings: &Settings,
    ui_control: &'static UiControlType<'_>,
    stack: Stack<'a>,
) -> WiFiController<'a> {
    info!("Starting WiFi AP network: {}", DEFAULT_AP_SSID);

    // Generate random password
    let password = {
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
    };

    let credentials = ScvCredentials {
        ssid: heapless::String::<32>::from_str(DEFAULT_AP_SSID).unwrap(),
        password,
    };
    let wifi_ap_data = ScWifiApData::WaitingForClient(credentials);
    let wifi_ap_screen = ScWifiAp::new(wifi_ap_data);
    ui_control
        .switch(ScCollection::WiFiAp(wifi_ap_screen))
        .await;

    state
}

/* Tasks */
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
