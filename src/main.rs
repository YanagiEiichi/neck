use clap::{Parser, Subcommand};

mod client;
mod http;
mod neck;
mod pool;
mod server;
mod utils;

#[derive(Parser, Debug)]
#[clap(name = "neck")]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Start an HTTP proxy server
    Serve {
        /// Binding the listeing address, defaults "0.0.0.0:1081"
        addr: Option<String>,
    },
    /// Create some worker connections and join the pool of the server
    Join {
        /// Proxy server address
        addr: String,
        /// Provided connections, default 16
        #[arg(short, long)]
        connections: Option<u16>,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();
    match args.command {
        Commands::Serve { addr } => {
            let mut a = addr.unwrap_or_else(|| String::from("0.0.0.0:1081"));
            // Convert port number to "0.0.0.0:{}"
            a = a
                .parse::<u16>()
                .map(|i| format!("0.0.0.0:{}", i))
                .unwrap_or(a);
            server::start(a).await;
        }
        Commands::Join { addr, connections } => {
            // Start client
            client::start(addr, connections.unwrap_or(16)).await
        }
    }
}
