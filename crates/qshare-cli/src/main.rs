use std::net::IpAddr;
use std::path::PathBuf;
use std::process::ExitCode;

use anyhow::{Context, Result};
use clap::Parser;
use qshare_core::{Server, ServerConfig};

#[derive(Parser, Debug)]
#[command(
    name = "qshare-cli",
    version,
    about = "Zero-config LAN file sharing — pick a folder, get a URL."
)]
struct Cli {
    /// Directory to share (defaults to current directory).
    /// Can also be passed positionally: `qshare-cli .`
    root: Option<PathBuf>,

    /// Directory to share (overrides positional).
    #[arg(long = "root", short = 'r', value_name = "ROOT")]
    root_flag: Option<PathBuf>,

    /// Port to listen on.
    #[arg(long, short = 'p', default_value_t = 8888)]
    port: u16,

    /// Host/IP to bind. Use 0.0.0.0 to expose on the LAN.
    #[arg(long, default_value = "0.0.0.0")]
    host: IpAddr,

    /// Show hidden files (those starting with `.`).
    #[arg(long, default_value_t = false)]
    show_hidden: bool,

    /// Directory listing cache TTL in seconds.
    #[arg(long, default_value_t = 5)]
    cache_ttl: u64,

    /// Don't print QR code in the terminal.
    #[arg(long, default_value_t = false)]
    no_qr: bool,
}

#[tokio::main(flavor = "multi_thread")]
async fn main() -> ExitCode {
    let cli = Cli::parse();

    let log_filter = tracing_subscriber::EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info,qshare_core=debug"));
    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .with_target(false)
        .init();

    if let Err(e) = run(cli).await {
        eprintln!("\n\x1b[31merror:\x1b[0m {e:?}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}

async fn run(cli: Cli) -> Result<()> {
    let root_raw = cli
        .root_flag
        .or(cli.root)
        .unwrap_or_else(|| PathBuf::from("."));
    let config = ServerConfig {
        root: root_raw
            .canonicalize()
            .with_context(|| format!("failed to resolve shared root: {}", root_raw.display()))?,
        host: cli.host,
        port: cli.port,
        show_hidden: cli.show_hidden,
        cache_ttl_secs: cli.cache_ttl,
        max_upload: 0, // unused in MVP (no upload)
    };

    let url = config.url();
    let root_path = config.root.clone();
    let server = Server::new(config)?;
    let handle = server.start().await?;

    print_banner(&url, &root_path, &handle, !cli.no_qr);

    let shutdown = wait_for_signal();
    tokio::select! {
        _ = shutdown => {
            eprintln!();
            eprintln!("\x1b[2mshutting down…\x1b[0m");
        }
    }
    handle.shutdown().await;
    Ok(())
}

fn print_banner(
    url: &str,
    root: &std::path::Path,
    handle: &qshare_core::ServerHandle,
    show_qr: bool,
) {
    let bar = "─".repeat(56);
    println!();
    println!("\x1b[36m  q-share\x1b[0m  {}", env!("CARGO_PKG_VERSION"));
    println!("\x1b[2m{}\x1b[0m", bar);
    println!("  shared  \x1b[2m{}\x1b[0m", root.display());
    println!("  url     \x1b[1;32m{}\x1b[0m", url);
    println!("  bind    \x1b[2m{}\x1b[0m", handle.local_addr());
    println!("\x1b[2m{}\x1b[0m", bar);
    println!("  press \x1b[1mCtrl+C\x1b[0m to stop");
    println!();
    if show_qr {
        if let Err(e) = print_qr_terminal(url) {
            eprintln!("\x1b[2m(qr render failed: {e})\x1b[0m");
        }
        println!();
    }
}

fn print_qr_terminal(url: &str) -> Result<()> {
    use qrcode::render::unicode;
    use qrcode::QrCode;

    let code = QrCode::new(url.as_bytes())?;
    let s = code.render::<unicode::Dense1x2>().quiet_zone(true).build();
    for line in s.lines() {
        println!("  {line}");
    }
    Ok(())
}

#[cfg(unix)]
async fn wait_for_signal() {
    use tokio::signal::unix::{signal, SignalKind};
    let mut sigint = signal(SignalKind::interrupt()).expect("install SIGINT");
    let mut sigterm = signal(SignalKind::terminate()).expect("install SIGTERM");
    tokio::select! {
        _ = sigint.recv() => {},
        _ = sigterm.recv() => {},
    }
}

#[cfg(not(unix))]
async fn wait_for_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
