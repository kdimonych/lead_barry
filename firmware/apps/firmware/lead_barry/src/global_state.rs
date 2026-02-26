#![allow(dead_code)]

use embassy_sync::lazy_lock::LazyLock;
use embassy_sync::{blocking_mutex::raw::CriticalSectionRawMutex, mutex::Mutex};

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum WiFiMode {
    None,
    Client,
    AccessPoint,
}

struct GlobalStateImpl {
    // Add any global state variables here if needed
    device_ip: Option<embassy_net::Ipv4Address>,
    wifi_mode: WiFiMode,
}

impl GlobalStateImpl {
    pub const fn new() -> Self {
        Self {
            device_ip: None,
            wifi_mode: WiFiMode::None,
        }
    }
}

pub struct GlobalState {
    inner: Mutex<CriticalSectionRawMutex, GlobalStateImpl>,
}

impl GlobalState {
    pub const fn new() -> Self {
        Self {
            inner: Mutex::new(GlobalStateImpl::new()),
        }
    }

    pub async fn set_device_ip(&self, ip: Option<embassy_net::Ipv4Address>) {
        self.inner.lock().await.device_ip = ip;
    }

    pub async fn get_device_ip(&self) -> Option<embassy_net::Ipv4Address> {
        let guard = self.inner.lock().await;
        guard.device_ip
    }

    pub async fn set_wifi_mode(&self, mode: WiFiMode) {
        self.inner.lock().await.wifi_mode = mode;
    }

    pub async fn get_wifi_mode(&self) -> WiFiMode {
        let guard = self.inner.lock().await;
        guard.wifi_mode
    }
}

pub fn global_state() -> &'static GlobalState {
    static GLOBAL_STATE: LazyLock<GlobalState> = LazyLock::new(GlobalState::new);
    GLOBAL_STATE.get()
}
