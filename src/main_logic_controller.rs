use defmt::*;

use cyw43::NetDriver;
use embassy_executor::Spawner;
use embassy_net::Stack;

use embassy_rp::clocks::RoscRng;
use embassy_time::Duration;
use embassy_time::Timer;

use crate::config_server::HttpConfigServer;
use crate::configuration::*;
use crate::input::*;
use crate::ui::*;
use crate::units::TimeExt as _;
use crate::vcp_sensors::*;
use crate::wifi::*;

// TODO: Move to separate module
// DHCP server
pub const VCP_SENSORS_EVENT_QUEUE_SIZE: usize = 8;

pub type VcpControlType<'a> = VcpControl<'a, VCP_SENSORS_EVENT_QUEUE_SIZE>;
pub type UiControlType<'a> = UiControl<'a, ScCollection>;

pub async fn main_logic_controller(
    spawner: Spawner,
    vcp_control: &'static VcpControlType<'_>,
    ui_control: &'static UiControlType<'_>,
    wifi_control: WiFiController<'static, IdleState>,
    wifi_network_driver: NetDriver<'static>,
    button_controller: ButtonController<'_>,
    configuration_storage: &'static ConfigurationStorage<'static>,
) -> ! {
    let set_screen = |new_screen: ScCollection| async { ui_control.switch(new_screen).await };
    let settings = configuration_storage.get_settings().await;

    let wifi_service = WiFiServiceBuilder::new(wifi_control, wifi_network_driver).build(spawner);
    let net_stack = wifi_service.net_stack().await;

    let mut network_ready = false;
    if !settings.network_settings.wifi_settings.ssid.is_empty() {
        wifi_service
            .join(&settings.network_settings.wifi_settings, async |status| {
                // Handle join status updates here
                info!("Join Status: {:?}", status);

                match status {
                    JoiningStatus::JoiningAP => {
                        let wifi_status = ScWifiStatsData::new(
                            ScvState::Connecting,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(ScWifiStats::new(wifi_status).into()).await;
                    }
                    JoiningStatus::ObtainingIP => {
                        let wifi_status: ScWifiStatsData = ScWifiStatsData::new(
                            ScvState::Dhcp,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(ScWifiStats::new(wifi_status).into()).await;
                    }
                    JoiningStatus::Ready => {
                        network_ready = true;
                        let wifi_status = ScWifiStatsData::new(
                            ScvState::Connected,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(ScWifiStats::new(wifi_status).into()).await;
                    }
                    JoiningStatus::Failed => {
                        error!("Failed to join WiFi network. Falling back to AP mode.");
                    }
                }
            })
            .await;
        info!("Joined WiFi network done");
        Timer::after(3.s()).await;
    }

    // If not joined, start AP mode
    if !network_ready {
        let mut wifi_ap_settings = settings.network_settings.wifi_ap_settings.clone();
        // Generate_random_password
        // TODO: Maybe it is  possible to eliminate clonong here
        wifi_ap_settings.password = Some(
            wifi_ap_settings
                .password
                .clone()
                .unwrap_or(generate_random_password()),
        );

        wifi_service
            .start_ap(
                &settings.network_settings.wifi_ap_settings,
                async |status| {
                    // Handle AP status updates here
                    info!("AP Status: {:?}", status);

                    match status {
                        ApStatus::StartingAP => {
                            // Set wifi ap screen with not ready state
                            let wifi_ap_data = ScWifiApData::NotReady;
                            set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
                        }
                        ApStatus::WaitingForClient => {
                            // Set wifi ap screen with not ready state
                            let wifi_ap_data = ScWifiApData::WaitingForClient(ScvCredentials {
                                ssid: wifi_ap_settings.ssid.clone(),
                                password: wifi_ap_settings.password.clone().unwrap_or_default(),
                            });
                            set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
                        }
                        ApStatus::Ready => {
                            //net_stack.
                            // Set wifi ap screen with not ready state
                            let wifi_ap_data = ScWifiApData::Connected(ScvClientInfo {
                                ip: wifi_ap_settings.ip.into(),
                                mac: None,
                            });
                            set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
                        }
                    }
                },
            )
            .await;
        info!("AP mode done");
        Timer::after(3.s()).await;
    };

    // Here we ready to start web server for configuration
    if let Some(net_cfg) = net_stack.config_v4() {
        let ip = net_cfg.address.address();

        let mut invitation = MessageString::complimentary_str();
        core::fmt::write(&mut invitation, format_args!("{} on your device.", ip)).ok();

        let msg = ScMessageData {
            title: MsgTitleString::from_str("Visit http://"),
            message: invitation.into(),
        };
        ui_control.switch(ScMessage::new(msg).into()).await;

        spawner
            .spawn(start_http_config_server(
                spawner,
                configuration_storage,
                net_stack,
            ))
            .unwrap();
    }
    loop {
        // Main logic goes here
        Timer::after(Duration::from_secs(60)).await;
    }
}

//HTTP configuration server task
#[embassy_executor::task]
async fn start_http_config_server(
    spawner: Spawner,
    configuration_storage: &'static ConfigurationStorage<'static>,
    stack: Stack<'static>,
) {
    let mut http_server = HttpConfigServer::new(spawner, configuration_storage);
    http_server.run(stack).await;
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

/* Tasks */
#[embassy_executor::task]
async fn net_task(mut runner: embassy_net::Runner<'static, cyw43::NetDriver<'static>>) -> ! {
    runner.run().await
}
