use core::mem::MaybeUninit;
use core::usize;

use defmt_or_log as log;

use ds323x::DateTimeAccess;
use ds323x::Timelike;
use embassy_executor::Spawner;
use embassy_futures::select::*;
use embassy_net::Stack;

use embassy_rp::clocks::RoscRng;
use embassy_sync::channel::Channel;
use embassy_sync::lazy_lock::LazyLock;
use embassy_time::Ticker;
use embassy_time::Timer;
use nanofish::HttpAllocator;
use static_cell::StaticCell;

use crate::configuration::*;
use crate::global_state::*;
use crate::input::*;
use crate::reset::trigger_system_reset;
use crate::rtc::*;
use crate::shared_resources::*;
use crate::ui::*;
use crate::units::TimeExt as _;
use crate::vcp_sensors::VcpSensorsEvents;
use crate::web_server::HttpConfigServer;
use crate::wifi::*;

const SOCKETS: usize = 3;
const HTTP_SERVER_WORKERS: usize = 1;
const HTTP_SERVER_BUFFER_SIZE: usize = HttpConfigServer::<SOCKETS>::MIN_SOCKET_POOL_BUFFER_SIZE;

static AP_STATUS_CHANNEL: StaticCell<ApStatusChannel> = StaticCell::new();
static HTTP_SERVER_BUFFER: StaticCell<[core::mem::MaybeUninit<u8>; HTTP_SERVER_BUFFER_SIZE]> = StaticCell::new();
static HTTP_SERVER_ALLOCATOR: StaticCell<HttpAllocator> = StaticCell::new();
static HTTP_SERVER: StaticCell<HttpConfigServer<'static, SOCKETS>> = StaticCell::new();

pub async fn main_logic_controller(
    spawner: Spawner,
    shared: &'static SharedResources,
    wifi_service: WifiService,
    button_controller: ButtonController<'_>,
) -> ! {
    let mut is_force_ap_mode_triggered = false;
    match detect_after_reset_actions(button_controller).await {
        AfterResetActions::FactoryReset => {
            do_factory_reset(shared.ui_control, shared.configuration_storage).await;
        }
        AfterResetActions::ApMode => {
            log::info!("Force AP mode was triggered after reset");
            is_force_ap_mode_triggered = true;
        }
        AfterResetActions::None => {
            log::info!("No special actions after reset");
        }
    }

    let set_screen = |new_screen: ScCollection| async { shared.ui_control.switch(new_screen).await };
    let settings = shared.configuration_storage.get_settings().await;

    let net_stack = wifi_service.net_stack().await;

    let mut network_ready = false;
    let is_wifi_configured = !settings.network_settings.wifi_settings.ssid.is_empty();
    let is_fallback_ap_set = settings.fallback_ap;
    let use_ap_mode = !is_wifi_configured || is_fallback_ap_set || is_force_ap_mode_triggered;

    // Flush button events to avoid misdetection after long operations
    button_controller.flush();

    log::debug!(
        "WiFi Configured: {}, Fallback AP: {}, Force AP: {}, Using AP mode: {}",
        is_wifi_configured,
        is_fallback_ap_set,
        is_force_ap_mode_triggered,
        use_ap_mode
    );

    if !use_ap_mode {
        wifi_service
            .join(&settings.network_settings.wifi_settings, async |status| {
                // Handle join status updates here
                log::info!("Join Status: {:?}", status);

                match status {
                    JoiningStatus::JoiningAP => {
                        let wifi_status = DmWifiStatus::new(
                            DmWifiStatusState::Connecting,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(wifi_status.into()).await;
                    }
                    JoiningStatus::Dhcp => {
                        let wifi_status: DmWifiStatus = DmWifiStatus::new(
                            DmWifiStatusState::Dhcp,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(wifi_status.into()).await;
                    }
                    JoiningStatus::Ready => {
                        network_ready = true;
                        let wifi_status = DmWifiStatus::new(
                            DmWifiStatusState::Connected,
                            Some(settings.network_settings.wifi_settings.ssid.clone()),
                        );
                        set_screen(wifi_status.into()).await;
                    }
                    JoiningStatus::Failed => {
                        log::error!("Failed to join WiFi network. Falling back to AP mode");
                        let msg = DmMessage {
                            title: MsgTitleString::from_str("ERROR"),
                            message: MessageString::from_str("Failed to join WiFi network. Starting AP..."),
                        };
                        set_screen(msg.into()).await;
                        Timer::after(2.s()).await;
                        shared
                            .configuration_storage
                            .modify_settings(|settings| {
                                settings.fallback_ap = true;
                            })
                            .await;
                        shared.configuration_storage.save().await.ok();
                        reboot_device(shared.ui_control).await;
                    }
                }
            })
            .await;

        if network_ready {
            log::info!("Joined WiFi network done");
            global_state().set_wifi_mode(WiFiMode::Client).await;
        }

        Timer::after(5.s()).await;
    }

    // If not joined, start AP mode
    if !network_ready {
        if settings.fallback_ap {
            log::info!("Starting in fallback AP mode as per settings");
            shared
                .configuration_storage
                .modify_settings(|settings| {
                    settings.fallback_ap = false;
                })
                .await;
            shared.configuration_storage.save().await.ok();
        } else {
            log::info!("Starting AP mode");
        }

        let wifi_ap_settings = settings.network_settings.wifi_ap_settings.clone();
        do_start_ap_mode(shared, &wifi_service, wifi_ap_settings, &button_controller).await;
        Timer::after(3.s()).await;
    };

    // Here we ready to start web server for configuration
    if let Some(net_cfg) = net_stack.config_v4() {
        let ip = net_cfg.address.address();
        global_state().set_device_ip(Some(ip)).await;

        let http_config_server = create_http_server(net_stack);

        for _ in 0..HTTP_SERVER_WORKERS {
            spawner
                .spawn(start_http_config_server(http_config_server, spawner, shared))
                .unwrap();
        }

        show_visit_screen(shared).await;
    }

    let mut channel: u8 = 0;

    let mut current_screan = button_controller.map_and_filter(button_event_to_screan).next().await;

    loop {
        match current_screan {
            ActiveScrean::TimeScreen => {
                current_screan = do_until_bt_action(&button_controller, || async {
                    show_time_screen(shared).await;
                })
                .await;
            }
            ActiveScrean::VoltageScreen => {
                log::debug!("Showing voltage for channel {}", channel);
                current_screan = on_repeat(
                    &current_screan,
                    do_until_bt_action(&button_controller, || async {
                        show_voltage_reading(shared, channel).await;
                    })
                    .await,
                    || async {
                        channel = (channel + 1) % 3;
                        log::debug!("Switching to voltage channel {}", channel);
                    },
                )
                .await;
            }
        }
    }
}

async fn do_start_ap_mode(
    shared: &'static SharedResources,
    wifi_service: &WifiService,
    mut wifi_ap_settings: WiFiApSettings,
    button_controller: &ButtonController<'_>,
) {
    let set_screen = |new_screen: ScCollection| async { shared.ui_control.switch(new_screen).await };

    // Generate random password if not set, to avoid having a blank password which can be a security risk
    wifi_ap_settings
        .password
        .get_or_insert_with(generate_random_password_uppercase);

    log::info!("Waiting for AP to start...");
    // Set wifi ap screen with not ready state
    let wifi_ap_data = DmWifiAp::NotReady;
    set_screen(wifi_ap_data.into()).await;

    let start_ap_status = AP_STATUS_CHANNEL.init_with(|| Channel::new());
    wifi_service
        .subtask_start_ap(&wifi_ap_settings, start_ap_status.sender())
        .await;

    // Wait for AP to start before allowing button interactions
    let status = start_ap_status.receive().await;
    match status {
        ApStatus::WaitingForClient => log::info!("AP is ready. Waiting for client to connect..."),
        ApStatus::Ready(_) => panic!("AP is already ready, expected to start in waiting for client state"),
    };

    // Set wifi ap screen with not ready state
    log::debug!("Waiting for client to connect...");
    log::debug!(
        "AP SSID: {}, Password: {}",
        wifi_ap_settings.ssid,
        wifi_ap_settings
            .password
            .as_ref()
            .map(|s| s.as_str())
            .unwrap_or("<empty>")
    );

    let set_wifi_credentials_screen = async move |use_qr_code: bool| {
        if use_qr_code {
            let mut qr_code_str = DmQrCodeString::complimentary_str();
            core::fmt::write(
                &mut qr_code_str,
                format_args!(
                    "WIFI:T:WPA;S:{};P:{};;",
                    wifi_ap_settings.ssid,
                    wifi_ap_settings.password.as_ref().unwrap_or(&heapless::String::new())
                ),
            )
            .unwrap();
            set_screen(DmQrCodeString::from_heapless(qr_code_str).into()).await;
        } else {
            let wifi_ap_data = DmWifiAp::WaitingForClient(DmWifiApCredentials {
                ssid: wifi_ap_settings.ssid.clone(),
                password: wifi_ap_settings.password.clone().unwrap_or_default(),
            });
            set_screen(wifi_ap_data.into()).await;
        }
    };

    let mut use_qr_code = false;

    let (ip, mac) = loop {
        set_wifi_credentials_screen(use_qr_code).await;

        let status_fut = start_ap_status.receive();
        let button_fut = button_controller.receive();
        let res = select(status_fut, button_fut).await;
        match res {
            Either::First(ApStatus::Ready((ip, mac))) => break (ip, mac),
            Either::First(ApStatus::WaitingForClient) => {
                panic!("Received unexpected AP status update: WaitingForClient when we were expecting Ready")
            }
            Either::Second(new_screan) => {
                // Toggle screen between QR code and credentials on button press
                if let ButtonEvent::Pressed(Buttons::Yellow) = new_screan {
                    use_qr_code = !use_qr_code;
                }
            }
        }
    };

    //net_stack.
    // Set wifi ap screen with not ready state
    log::trace!("Ap ready. Client connected.");
    // network_ready = true;
    let wifi_ap_data = DmWifiAp::Connected(DmWifiApClientInfo { ip, mac: Some(mac) });
    set_screen(wifi_ap_data.into()).await;

    wifi_service.wait_for_subtask_finish().await;

    // Handle AP status updates here
    global_state().set_wifi_mode(WiFiMode::AccessPoint).await;
    log::info!("AP mode done");
}

fn button_event_to_screan(event: &ButtonEvent) -> Option<ActiveScrean> {
    match event {
        ButtonEvent::Pressed(Buttons::Yellow) => Some(ActiveScrean::TimeScreen),
        ButtonEvent::Pressed(Buttons::Blue) => Some(ActiveScrean::VoltageScreen),
        _ => None,
    }
}

#[derive(PartialEq)]
enum ActiveScrean {
    TimeScreen,
    VoltageScreen,
}

async fn on_repeat<F, Fut>(old: &ActiveScrean, new: ActiveScrean, f: F) -> ActiveScrean
where
    F: FnOnce() -> Fut,
    Fut: core::future::Future<Output = ()>,
{
    if *old == new {
        f().await;
        new
    } else {
        new
    }
}

async fn do_until_bt_action<F, Fut>(button_controller: &ButtonController<'_>, mut f: F) -> ActiveScrean
where
    F: FnMut() -> Fut,
    Fut: core::future::Future<Output = core::convert::Infallible>,
{
    let res =
        embassy_futures::select::select(button_controller.map_and_filter(button_event_to_screan).next(), f()).await;
    match res {
        Either::First(new_screan) => new_screan,
        Either::Second(_) => log::unreachable!(),
    }
}

async fn show_time_screen(shared: &'static SharedResources) -> ! {
    let mut ticker = Ticker::every(1.s());

    let mut time_str = MessageString::complimentary_str();
    let show_time = async |time_str: &heapless::String<_>| {
        let msg = DmMessage {
            title: MsgTitleString::from_str("Current Time"),
            message: time_str.clone().into(),
        };
        shared.ui_control.switch(msg.into()).await;
    };

    let update_time_str = async |time_str: &mut heapless::String<_>| {
        let mut rtc = shared.rtc.lock().await;

        let mut t = None;
        if let Ok(false) = rtc.busy().await {
            rtc.convert_temperature().await.ok();
            t = rtc.temperature().await.ok();
        }

        if let Ok(datetime) = rtc.datetime().await {
            time_str.clear();
            core::fmt::write(
                time_str,
                format_args!(
                    "{:04}-{:02}-{:02}\n{:02}:{:02}:{:02}\nt: {:.01} C",
                    datetime.year(),
                    datetime.month(),
                    datetime.day(),
                    datetime.hour(),
                    datetime.minute(),
                    datetime.second(),
                    t.unwrap_or_default()
                ),
            )
            .ok();
        };
    };
    loop {
        update_time_str(&mut time_str).await;
        show_time(&time_str).await;
        ticker.next().await;
    }
}

async fn show_voltage_reading(shared: &'static SharedResources, channel: u8) -> ! {
    let mut ticker = Ticker::every(40.ms());

    // Pick a channel to monitor
    shared.vcp_control.disable_all_channels().await;
    shared.vcp_control.enable_channel(channel).await;
    shared.vcp_control.flush_events();

    static VOLTAGE: LazyLock<SharedDataModel<f32>> = LazyLock::new(|| SharedDataModel::new(0f32));
    let voltage = VOLTAGE.get();

    let mut title = DmVcpTitle::complimentary_str();
    core::fmt::write(&mut title, format_args!("Channel {}", channel + 1)).ok();

    let vcp = DmVcp::new(voltage, DmVcpBaseUnits::Volts, title.into());
    shared.ui_control.switch(vcp.into()).await;

    // Voltage update loop
    loop {
        if let VcpSensorsEvents::Reading(reading) = shared.vcp_control.receive_event().await
            && reading.channel == channel
        {
            *voltage.lock().await = reading.voltage.value();
        }
        ticker.next().await;
    }
}

async fn show_visit_screen(shared: &'static SharedResources) {
    if let Some(ip) = global_state().get_device_ip().await {
        let mut invitation = MessageString::complimentary_str();
        core::fmt::write(&mut invitation, format_args!("http://\n{} on your device.", ip)).ok();

        let msg = DmMessage {
            title: MsgTitleString::from_str("Visit"),
            message: invitation.into(),
        };
        shared.ui_control.switch(msg.into()).await;
    }
}

/// Helper function to initialize the HTTP allocator with the correct generic parameters,
/// since Rust doesn't allow using const generics in async functions directly
fn init_http_allocator() -> &'static mut HttpAllocator<'static> {
    let buffer = HTTP_SERVER_BUFFER.init_with(|| bump_into::space_uninit!(HTTP_SERVER_BUFFER_SIZE));
    HTTP_SERVER_ALLOCATOR.init_with(|| HttpAllocator::from_slice(buffer))
}

/// Helper function to create the HTTP server with the correct generic parameters,
/// since Rust doesn't allow using const generics in async functions directly
#[inline(always)]
fn create_http_server(stack: Stack<'static>) -> &'static HttpConfigServer<'static, SOCKETS> {
    HTTP_SERVER.init_with(|| HttpConfigServer::<'static, SOCKETS>::new(init_http_allocator(), stack))
}

//HTTP configuration server task
#[embassy_executor::task(pool_size = HTTP_SERVER_WORKERS)]
async fn start_http_config_server(
    server: &'static HttpConfigServer<'static, SOCKETS>,
    spawner: Spawner,
    shared: &'static SharedResources,
) {
    // Initialize the worker allocator for this task
    let mut worker_buffer = [MaybeUninit::<u8>::uninit(); HttpConfigServer::<SOCKETS>::MIN_WORKER_BUFFER_SIZE];

    // Start the HTTP server
    server.run(&mut worker_buffer, spawner, shared).await;
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

async fn do_factory_reset(
    ui_control: &UiControl<'_>,
    configuration_storage: &'static ConfigurationStorage<'static>,
) -> bool {
    let msg = DmMessage {
        title: MsgTitleString::from_str("Factory Reset"),
        message: MessageString::from_str("Performing factory reset..."),
    };
    ui_control.switch(msg.into()).await;
    let res = if let Err(e) = configuration_storage.factory_reset().await {
        log::error!("Factory reset failed: {:?}", e);
        let msg = DmMessage {
            title: MsgTitleString::from_str("ERROR"),
            message: MessageString::from_str("Factory reset failed."),
        };
        ui_control.switch(msg.into()).await;
        false
    } else {
        log::info!("Factory reset completed successfully");
        let msg = DmMessage {
            title: MsgTitleString::from_str("INFO"),
            message: MessageString::from_str("Factory reset completed successfully."),
        };
        ui_control.switch(msg.into()).await;
        true
    };
    Timer::after(3.s()).await;
    res
}

enum AfterResetActions {
    None,
    ApMode,
    FactoryReset,
}

async fn detect_after_reset_actions(button_controller: ButtonController<'_>) -> AfterResetActions {
    let y_state = button_controller.get_last_state(Buttons::Yellow).await.unwrap();
    let b_state = button_controller.get_last_state(Buttons::Blue).await.unwrap();

    if y_state == ButtonState::Pressed && b_state == ButtonState::Pressed {
        log::info!("Factory reset was triggered");
        return AfterResetActions::FactoryReset;
    } else if y_state == ButtonState::Pressed {
        log::info!("AP mode was triggered");
        return AfterResetActions::ApMode;
    }
    AfterResetActions::None
}

async fn reboot_device(ui_control: &UiControl<'_>) -> ! {
    let msg = DmMessage {
        title: MsgTitleString::from_str("Rebooting"),
        message: MessageString::from_str("The device is rebooting..."),
    };
    ui_control.switch(msg.into()).await;
    Timer::after(2.s()).await;
    trigger_system_reset()
}
