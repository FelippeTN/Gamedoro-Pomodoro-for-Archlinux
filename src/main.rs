//! Gamedoro: pomodoro ultraleve para Waybar com trilhas de jogos (YouTube → cache local).
//! Composition root: liga o domínio/aplicação aos adapters concretos.

use gamedoro::adapters::config_file::FileStore;
use gamedoro::adapters::desktop::Desktop;
use gamedoro::adapters::local::LocalPlayer;
use gamedoro::app::{Command, Pomodoro};
use gamedoro::daemon;
use std::process::ExitCode;

const USAGE: &str = "uso: gamedoro                      (daemon, iniciado pela waybar)
     gamedoro toggle|skip|reset
     gamedoro focus ±N | break ±N     (minutos)
     gamedoro music toggle|next|prev|cycle|pack <minecraft|witcher3|tlou>
     gamedoro vol ±N | vol N          (volume relativo | absoluto, porcento)
     gamedoro-menu                    (menu de controle; o clique direito no timer abre)";

fn build() -> Pomodoro {
    let (cfg, store) = FileStore::open();
    Pomodoro::new(cfg.settings, Box::new(LocalPlayer::new(cfg.sources)), Box::new(Desktop), Box::new(store))
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
