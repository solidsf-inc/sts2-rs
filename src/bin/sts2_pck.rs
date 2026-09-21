use clap::Parser;
use std::path::PathBuf;
use sts2_rs::pck::extract_localization;

#[derive(Parser, Debug)]
#[command(name = "sts2-pck")]
#[command(
    about = "Native high-speed Godot PCK unpacker and checksum validator for Slay the Spire 2"
)]
struct Cli {
    /// Path to the game PCK file
    pck: PathBuf,

    /// Project root directory to output localization files into
    #[arg(short, long, default_value = ".")]
    root: PathBuf,
}

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    println!("📦 Extracting localization from {}...", cli.pck.display());
    let count = extract_localization(&cli.pck, &cli.root)?;
    println!("✅ Successfully refreshed {} localization tables.", count);
    Ok(())
}
