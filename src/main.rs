//! Gamedoro: pomodoro ultraleve para Waybar com trilhas de jogos (YouTube → cache local).
//! Composition root: liga o domínio/aplicação aos adapters concretos.

mod adapters;
mod app;
mod daemon;
mod domain;
mod ports;

use adapters::config_file::FileStore;
use adapters::desktop::Desktop;
use adapters::local::LocalPlayer;
use app::{Command, Pomodoro};
use std::process::ExitCode;

const USAGE: &str = "uso: gamedoro                      (daemon, iniciado pela waybar)
     gamedoro toggle|skip|reset
     gamedoro focus ±N | break ±N     (minutos)
     gamedoro music toggle|next|prev|cycle|pack <minecraft|witcher3|tlou>
     gamedoro vol ±N                  (porcento)
     gamedoro rofi                    (menu de controle, script mode do rofi)";

fn build() -> Pomodoro {
    let (cfg, store) = FileStore::open();
    Pomodoro::new(cfg.settings, Box::new(LocalPlayer::new(cfg.sources)), Box::new(Desktop), Box::new(store))
}

fn main() -> ExitCode {
    let line = std::env::args().skip(1).collect::<Vec<_>>().join(" ");
    if line.is_empty() {
        daemon::run(build);
    }
    // o rofi chama `gamedoro rofi <texto do item>`: o item escolhido vem por ROFI_INFO, o texto é ignorado
    if std::env::args().nth(1).as_deref() == Some("rofi") {
        adapters::rofi::run();
        return ExitCode::SUCCESS;
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
