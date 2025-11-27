use defmt::*;
use embassy_executor::Spawner;
use embassy_futures::join::join;
use embassy_net::{Ipv4Address, Stack};
use embassy_sync::blocking_mutex::raw::CriticalSectionRawMutex;
use embassy_sync::mutex::Mutex;
use embassy_sync::signal::Signal;
use leasehund::{
    DHCPServerBuffers, DHCPServerSocket, DhcpServer as LhDhcpServer, TransactionEvent,
};

const MAX_CLIENTS: usize = 2;
const MAX_DNS_SERVERS: usize = 1;

enum DhcpServerCmmand {
    Stop,
}

#[derive(PartialEq)]
enum DhcpServerEvent {
    Stopped,
    Started,
    /// Indicates a new lease assignment
    Leased(Ipv4Address, [u8; 6]),
    /// Indicates a release the IP by a client
    Released(Ipv4Address, [u8; 6]),
}
//    const MAX_CLIENTS: usize = DEFAULT_MAX_CLIENTS,
//    const MAX_DNS: usize = DEFAULT_MAX_DNS_SERVERS,
pub struct DhcpServerState {
    command: Signal<CriticalSectionRawMutex, DhcpServerCmmand>,
    event: Signal<CriticalSectionRawMutex, DhcpServerEvent>,
    dhcp_server: Mutex<CriticalSectionRawMutex, Option<LhDhcpServer<MAX_CLIENTS, MAX_DNS_SERVERS>>>,
}

impl DhcpServerState {
    pub fn new() -> Self {
        let res = Self {
            command: Signal::new(),
            event: Signal::new(),
            dhcp_server: Mutex::new(None),
        };

        // Initial state is stopped
        res.event.signal(DhcpServerEvent::Stopped);
        res
    }
}

pub struct DhcpServer {
    state: &'static DhcpServerState,
}

pub struct DhcpServerConfig {
    server_ip: Ipv4Address,
    subnet_mask: Ipv4Address,
    router: Ipv4Address,
    ip_pool_start: Ipv4Address,
    ip_pool_end: Ipv4Address,
}

impl DhcpServerConfig {
    pub const fn new(
        server_ip: Ipv4Address,
        subnet_mask: Ipv4Address,
        router: Ipv4Address,
        ip_pool_start: Ipv4Address,
        ip_pool_end: Ipv4Address,
    ) -> Self {
        Self {
            server_ip,
            subnet_mask,
            router,
            ip_pool_start,
            ip_pool_end,
        }
    }
}

impl Default for DhcpServerConfig {
    fn default() -> Self {
        Self {
            server_ip: Ipv4Address::new(192, 168, 1, 1),
            subnet_mask: Ipv4Address::new(255, 255, 255, 0),
            router: Ipv4Address::new(192, 168, 1, 1),
            ip_pool_start: Ipv4Address::new(192, 168, 1, 2),
            ip_pool_end: Ipv4Address::new(192, 168, 1, 255),
        }
    }
}

pub enum DhcpEvent {
    Lease(Ipv4Address, [u8; 6]),
    Release(Ipv4Address, [u8; 6]),
}

impl DhcpServer {
    pub async fn new(state: &'static DhcpServerState) -> Self {
        state.command.signal(DhcpServerCmmand::Stop);
        while state.event.wait().await != DhcpServerEvent::Stopped {}
        // Destroy existing server, if existing
        state.dhcp_server.lock().await.take();
        state.command.reset();

        Self { state }
    }

    pub async fn wait_event(&self) -> Option<DhcpEvent> {
        match self.state.event.wait().await {
            DhcpServerEvent::Leased(ip, mac) => Some(DhcpEvent::Lease(ip, mac)),
            DhcpServerEvent::Released(ip, mac) => Some(DhcpEvent::Release(ip, mac)),
            event => {
                self.state.event.signal(event);
                None
            } // Re-signal other events
        }
    }

    pub async fn start(
        &self,
        spawner: Spawner,
        stack: Stack<'static>,
        dhcp_config: DhcpServerConfig,
    ) {
        // Stop existing server, if existing
        self.state.command.signal(DhcpServerCmmand::Stop);
        while self.state.event.wait().await != DhcpServerEvent::Stopped {}
        // Destroy existing server, if existing
        self.state.dhcp_server.lock().await.take();
        self.state.command.reset();

        let server = LhDhcpServer::new(
            dhcp_config.server_ip,     // Server IP
            dhcp_config.subnet_mask,   // Subnet mask
            dhcp_config.router,        // Gateway
            Ipv4Address::UNSPECIFIED,  // DNS server
            dhcp_config.ip_pool_start, // Pool start
            dhcp_config.ip_pool_end,   // Pool end
        );

        *self.state.dhcp_server.lock().await = Some(server);

        //Spawn DHCP server task
        while spawner.spawn(dhcp_server_task(self.state, stack)).is_err() {}
    }
    pub async fn stop(&self) {
        self.state.command.signal(DhcpServerCmmand::Stop);
        while self.state.event.wait().await != DhcpServerEvent::Stopped {}
        // Destroy existing server, if existing
        self.state.dhcp_server.lock().await.take();
        self.state.command.reset();
    }
}

/* Tasks */
#[embassy_executor::task]
async fn dhcp_server_task(state: &'static DhcpServerState, stack: Stack<'static>) {
    //let cmd = state.command.wait().await;

    if let Some(dhcp_server) = state.dhcp_server.lock().await.as_mut() {
        info!("Starting DHCP server task");
        state.event.signal(DhcpServerEvent::Started);

        let mut buffers = DHCPServerBuffers::new();
        let mut socket = DHCPServerSocket::new(stack, &mut buffers);

        loop {
            match join(state.event.wait(), dhcp_server.lease_one(&mut socket)).await {
                (DhcpServerEvent::Stopped, _) => {
                    info!("Stopping DHCP server task");
                    break;
                }
                (_, Ok(TransactionEvent::Leased(ip, mac))) => {
                    info!("Leased IP: {} for MAC: {}", ip, mac);
                    // Wait a bit before returning to let the stack send the ACK packet
                    state.event.signal(DhcpServerEvent::Leased(ip, mac));
                }
                (_, Ok(TransactionEvent::Released(ip, mac))) => {
                    info!("Released IP: {} for MAC: {}", ip, mac);
                    state.event.signal(DhcpServerEvent::Released(ip, mac));
                }
                (_, Err(e)) => {
                    error!("DHCP server error: {:?}", e);
                    embassy_futures::yield_now().await;
                }
            }
        }
    } else {
        error!("DHCP server instance not found, stopping DHCP server task");
    }

    state.event.signal(DhcpServerEvent::Stopped);
}
