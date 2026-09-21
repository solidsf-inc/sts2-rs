use clap::Parser;
use std::path::PathBuf;
use sts2_rs::bridge::BridgeServer;
use sts2_rs::driver::{MockDriver, SubprocessDriver};

#[derive(Parser, Debug)]
#[command(name = "sts2-bridge")]
#[command(about = "Native HTTP bridge server connecting external agents to STS2 headless process")]
struct Cli {
    /// Port to listen on
    #[arg(short, long, default_value_t = 9876)]
    port: u16,

    /// Project root path
    #[arg(short, long, default_value = "..")]
    root: PathBuf,

    /// Force mock driver mode
    #[arg(long)]
    mock: bool,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let abs_root = std::fs::canonicalize(&cli.root).unwrap_or_else(|_| cli.root.clone());

    if cli.mock {
        println!("🎮 Running in Mock Driver Mode");
        let driver = MockDriver::new();
        let bridge = BridgeServer::new(driver, cli.port);
        bridge.run()?;
    } else {
        match SubprocessDriver::new(&abs_root) {
            Ok(driver) => {
                let bridge = BridgeServer::new(driver, cli.port);
                bridge.run()?;
            }
            Err(e) => {
                eprintln!(
                    "⚠️  Could not launch live game process ({}). Falling back to mock driver.",
                    e
                );
                let driver = MockDriver::new();
                let bridge = BridgeServer::new(driver, cli.port);
                bridge.run()?;
            }
        }
    }

    Ok(())
}
