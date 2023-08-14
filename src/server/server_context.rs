use super::{Hub, MockPool, Pool};

pub struct ServerContext {
    pub addr: String,
    pub pool: Box<dyn Hub>,
}

impl ServerContext {
    /// Creates a new [`ServerContext`].
    pub fn new(addr: Option<String>, direct: bool) -> Self {
        Self {
            addr: fix_addr(addr),
            pool: if direct {
                Box::new(MockPool {})
            } else {
                Box::new(Pool::new())
            },
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
