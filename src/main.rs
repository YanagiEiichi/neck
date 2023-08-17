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

        /// The number of maximum provided connections defaults 200
        #[arg(short, long)]
        connections: Option<u32>,

        /// The number of concurrent workers defaults 8.
        #[arg(short, long)]
        workers: Option<u32>,

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
            workers,
            tls,
            tls_domain,
        } => {
            // Start client
            NeckClient::new(addr, workers, connections, tls, tls_domain)
                .start()
                .await;
        }
    }
}
