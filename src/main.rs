//! Gamedoro: pomodoro ultraleve para Waybar com trilhas de jogos (YouTube → cache local).
//! Composition root: liga o domínio/aplicação aos adapters concretos.

mod adapters;
mod app;
mod daemon;
mod domain;
mod ports;

use adapters::config_file::{ConfigFile, FileStore};
use adapters::desktop::Desktop;
use adapters::local::LocalPlayer;
use app::{Command, Pomodoro};
use std::process::ExitCode;

const USAGE: &str = "uso: gamedoro                      (daemon, iniciado pela waybar)
     gamedoro toggle|skip|reset
     gamedoro focus ±N | break ±N     (minutos)
     gamedoro music toggle|next|prev|cycle|pack <minecraft|witcher3|tlou>
     gamedoro vol ±N                  (porcento)";

fn build() -> Pomodoro {
    let cfg = ConfigFile::load();
    Pomodoro::new(
        cfg.settings,
        Box::new(LocalPlayer::new(cfg.sources.clone())),
        Box::new(Desktop),
        Box::new(FileStore::new(cfg.sources)),
    )
}

fn main() -> ExitCode {
    let line = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        daemon::run(build);
    }
    if matches!(line.as_str(), "-h" | "--help" | "help") {
        println!("{USAGE}");
        return ExitCode::SUCCESS;
    }
    if let Err(e) = Command::parse(&line) {
        eprintln!("gamedoro: {e}\n\n{USAGE}");
        return ExitCode::FAILURE;
    }
    match daemon::send(&line) {
        Ok(()) => ExitCode::SUCCESS,
        Err(_) => {
            eprintln!("gamedoro: daemon não está rodando (ele é iniciado pela waybar)");
            ExitCode::FAILURE
        }
    }
}
