use std::process::ExitCode;

use clap::Parser;
use fawkes_probe::adapters::openai::OpenAiVisionClient;
use fawkes_probe::adapters::screen::XcapScreenCapture;
use fawkes_probe::adapters::store::SqliteCaptureStore;
use fawkes_probe::adapters::time::SystemProbeTime;
use fawkes_probe::application::service::ProbeRunService;
use fawkes_probe::cli::CliArgs;
use fawkes_probe::{ProbeError, ProbeRunConfig};

fn main() -> ExitCode {
    match run_probe_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

fn run_probe_main() -> Result<(), ProbeError> {
    let args = CliArgs::parse();
    let config = ProbeRunConfig::parse_strict_probe_args(args)?;
    let store = SqliteCaptureStore::open_local_probe_store(config.output_dir().to_path_buf())?;
    let screen = XcapScreenCapture;
    let vision = OpenAiVisionClient::new(
        config.openai_api_key().clone(),
        config.model_name().to_owned(),
        None,
    )?;
    let time = SystemProbeTime;
    let service = ProbeRunService::new(screen, vision, store, time);
    let summary = service.run_serial_probe_cycle(&config)?;
    print!("{}", summary.render_terminal_summary_report());
    Ok(())
}
