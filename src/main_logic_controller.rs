use defmt::*;

use cyw43::NetDriver;
use embassy_executor::Spawner;
use embassy_net::Stack;

use embassy_rp::clocks::RoscRng;
use embassy_time::Duration;
use embassy_time::Timer;

use crate::configuration::*;
use crate::input::*;
use crate::reset::trigger_system_reset;
use crate::shared_resources::*;
use crate::ui::*;
use crate::units::TimeExt as _;
use crate::vcp_sensors::*;
use crate::web_server::HttpConfigServer;
use crate::wifi::*;

pub async fn main_logic_controller(
    spawner: Spawner,
    vcp_control: &'static VcpControl<'_>,
    ui_control: &'static UiControl<'_>,
    wifi_service: WifiService,
    button_controller: ButtonController<'_>,
    configuration_storage: &'static ConfigurationStorage<'static>,
) -> ! {
    // Check if factory reset is triggered and perform it if needed
    factory_reset_if_triggered(button_controller, ui_control, configuration_storage).await;

    let set_screen = |new_screen: ScCollection| async { ui_control.switch(new_screen).await };
    let settings = configuration_storage.get_settings().await;

    let net_stack = wifi_service.net_stack().await;

    let mut network_ready = false;
    if !(settings.network_settings.wifi_settings.ssid.is_empty() || settings.fallback_ap) {
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
                        error!("Failed to join WiFi network. Falling back to AP mode");
                        let msg = ScMessageData {
                            title: MsgTitleString::from_str("ERROR"),
                            message: MessageString::from_str(
                                "Failed to join WiFi network. Starting AP...",
                            ),
                        };
                        set_screen(ScMessage::new(msg).into()).await;
                        Timer::after(2.s()).await;
                        configuration_storage
                            .modify_settings(|settings| {
                                settings.fallback_ap = true;
                            })
                            .await;
                        configuration_storage.save().await.ok();
                        reboot_device(ui_control).await;
                    }
                }
            })
            .await;
        info!("Joined WiFi network done");

        Timer::after(5.s()).await;
    }

    // If not joined, start AP mode
    if !network_ready {
        if (settings.fallback_ap) {
            info!("Starting in fallback AP mode as per settings");
            configuration_storage
                .modify_settings(|settings| {
                    settings.fallback_ap = false;
                })
                .await;
            configuration_storage.save().await.ok();
        } else {
            info!("Starting AP mode");
        }

        let mut wifi_ap_settings = settings.network_settings.wifi_ap_settings.clone();
        // Generate_random_password
        // TODO: Maybe it is  possible to eliminate clonong here
        wifi_ap_settings.password = Some(
            wifi_ap_settings
                .password
                .clone()
                .unwrap_or(generate_random_password_uppercase()),
        );

        wifi_service
            .start_ap(&wifi_ap_settings, async |status| {
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
                        debug!("Waiting for client to connect...");
                        debug!(
                            "AP SSID: {}, Password: {}",
                            wifi_ap_settings.ssid,
                            wifi_ap_settings
                                .password
                                .as_ref()
                                .map(|s| s.as_str())
                                .unwrap_or("<empty>")
                        );
                        let wifi_ap_data = ScWifiApData::WaitingForClient(ScvCredentials {
                            ssid: wifi_ap_settings.ssid.clone(),
                            password: wifi_ap_settings.password.clone().unwrap_or_default(),
                        });
                        set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
                    }
                    ApStatus::Ready((ip, mac)) => {
                        //net_stack.
                        // Set wifi ap screen with not ready state
                        let wifi_ap_data =
                            ScWifiApData::Connected(ScvClientInfo { ip, mac: Some(mac) });
                        set_screen(ScWifiAp::new(wifi_ap_data).into()).await;
                    }
                }
            })
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

fn generate_random_password_uppercase() -> heapless::String<64> {
    let mut rng = RoscRng;
    let mut pwd = heapless::String::<64>::new();
    for _ in 0..8 {
        let idx = (rng.next_u32() % 35) as u8;
        let c = if idx < 9 {
            (b'1' + idx) as char
        } else {
            (b'A' + idx - 9) as char
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

async fn factory_reset_if_triggered(
    button_controller: ButtonController<'_>,
    ui_control: &UiControl<'_>,
    configuration_storage: &'static ConfigurationStorage<'static>,
) {
    let y_state = button_controller
        .get_last_state(Buttons::Yellow)
        .await
        .unwrap();
    let b_state = button_controller
        .get_last_state(Buttons::Blue)
        .await
        .unwrap();

    if y_state == ButtonState::Pressed && b_state == ButtonState::Pressed {
        info!("Factory reset was triggered");
        let msg = ScMessageData {
            title: MsgTitleString::from_str("Factory Reset"),
            message: MessageString::from_str("Performing factory reset..."),
        };
        ui_control.switch(ScMessage::new(msg).into()).await;
        if let Err(e) = configuration_storage.factory_reset().await {
            error!("Factory reset failed: {:?}", e);
            let msg = ScMessageData {
                title: MsgTitleString::from_str("ERROR"),
                message: MessageString::from_str("Factory reset failed."),
            };
            ui_control.switch(ScMessage::new(msg).into()).await;
        } else {
            info!("Factory reset completed successfully");
            let msg = ScMessageData {
                title: MsgTitleString::from_str("INFO"),
                message: MessageString::from_str("Factory reset completed successfully."),
            };
            ui_control.switch(ScMessage::new(msg).into()).await;
        }
        Timer::after(3.s()).await;
    }
}

async fn reboot_device(ui_control: &UiControl<'_>) -> ! {
    let msg = ScMessageData {
        title: MsgTitleString::from_str("Rebooting"),
        message: MessageString::from_str("The device is rebooting..."),
    };
    ui_control.switch(ScMessage::new(msg).into()).await;
    Timer::after(2.s()).await;
    trigger_system_reset()
}
