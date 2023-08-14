use super::{
    connection_manager::ConnectionManager, direct_mode_manager::DirectModeManager,
    pool_mode_manager::PoolModeManager,
};

pub struct ServerContext {
    pub addr: String,
    pub manager: Box<dyn ConnectionManager>,
}

impl ServerContext {
    /// Creates a new [`ServerContext`].
    pub fn new(addr: Option<String>, direct: bool) -> Self {
        Self {
            addr: fix_addr(addr),
            manager: if direct {
                Box::new(DirectModeManager {})
            } else {
                Box::new(PoolModeManager::new())
            },
        }
    }
}

fn fix_addr(addr: Option<String>) -> String {
    addr.map_or_else(
        // Get addr, use "0.0.0.0:1081" as the default valeu.
        || String::from("0.0.0.0:1081"),
        // Convert pure number {port} to "0.0.0.0:{port}"
        |v| v.parse::<u16>().map_or(v, |i| format!("0.0.0.0:{}", i)),
    )
}
