use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Clone, Parser)]
#[command(name = "fawkes_probe")]
#[command(about = "Probe screen focus against a declared goal using OpenAI vision.")]
pub struct CliArgs {
    #[arg(long)]
    pub goal: String,
    #[arg(long)]
    pub interval: u64,
    #[arg(long)]
    pub count: u32,
    #[arg(long, default_value = "gpt-4.1-mini")]
    pub model: String,
    #[arg(long, default_value = ".fawkes_probe")]
    pub output_dir: PathBuf,
}
