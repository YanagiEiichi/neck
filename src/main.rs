use clap::{Parser, Subcommand};
use client::NeckClient;
use server::NeckServer;

mod client;
mod http;
mod neck;
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
    /// Start a Neck HTTP proxy server
    Serve {
        /// Binding the listening address defaults "0.0.0.0:1081"
        addr: Option<String>,

        /// Proxy directly from the server without creating a worker pool.
        #[clap(long, action)]
        direct: bool,
    },
    /// Create some worker connections and join the pool of the server
    Join {
        /// Proxy server address
        addr: String,

        /// The provided connections defaults 100
        #[arg(short, long)]
        connections: Option<u64>,

        /// Connect proxy server using TLS.
        #[clap(long, action)]
        tls: bool,

        /// Specify the domain for TLS, using the hostname of addr by default.
        #[arg(long)]
        tls_domain: Option<String>,
    },
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    match args.command {
        Commands::Serve { addr, direct } => {
            NeckServer::new(addr, direct).start().await;
        }

        Commands::Join {
            addr,
            connections,
            tls,
            tls_domain,
        } => {
            // Start client
            NeckClient::new(addr, connections, tls, tls_domain)
                .start()
                .await;
        }
    }
}
