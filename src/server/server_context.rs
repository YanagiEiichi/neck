use super::Pool;

pub struct ServerContext {
    pub addr: String,
    pub pool: Pool,
}

impl ServerContext {
    /// Creates a new [`ServerContext`].
    pub fn new(addr: Option<String>) -> Self {
        Self {
            addr: fix_addr(addr),
            pool: Pool::new(),
        }
    }
}

fn fix_addr(addr: Option<String>) -> String {
    // Get addr, use "0.0.0.0:1081" as the default valeu.
    addr.map_or_else(
        || String::from("0.0.0.0:1081"),
        |v| {
            // Convert pure number {port} to "0.0.0.0:{port}"
            v.parse::<u16>().map_or(v, |i| format!("0.0.0.0:{}", i))
        },
    )
}
