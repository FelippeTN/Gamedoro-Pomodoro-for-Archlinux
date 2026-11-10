//! Slider de volume arrastável com o mouse (`zenity --scale`). O rofi não tem slider, então o
//! menu abre este diálogo. Ele aplica o volume em tempo real enquanto você arrasta
//! (`--print-partial`), "Cancelar" volta ao valor anterior, e só existe enquanto está aberto.

use crate::adapters::config_file::ConfigFile;
use crate::daemon;
use std::io::{BufRead, BufReader};
use std::process::{Command, Stdio};

pub fn run() {
    let start = ConfigFile::load().settings.volume.percent();
    let spawned = Command::new("zenity")
        .args(["--scale", "--title", "Gamedoro", "--text", "\u{f057e}  Volume da música"])
        .args(["--min-value", "0", "--max-value", "100", "--step", "1", "--print-partial"])
        .arg(format!("--value={start}"))
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn();
    let Ok(mut zenity) = spawned else {
        eprintln!("gamedoro: instale o zenity para o slider de volume (sudo pacman -S zenity)");
        return;
    };
    let mut last = start;
    if let Some(out) = zenity.stdout.take() {
        for value in BufReader::new(out).lines().map_while(Result::ok).filter_map(|l| percent(&l)) {
            if value != last && daemon::send(&format!("vol {value}")).is_ok() {
                last = value;
            }
        }
    }
    let confirmed = zenity.wait().is_ok_and(|s| s.success());
    if !confirmed && last != start {
        let _ = daemon::send(&format!("vol {start}")); // Cancelar / fechar: desfaz
    }
}

/// Linha do zenity → porcentagem válida (ignora qualquer outra coisa).
fn percent(line: &str) -> Option<u32> {
    line.trim().parse().ok().filter(|v| *v <= 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_zenity_values() {
        assert_eq!(percent("65\n"), Some(65));
        assert_eq!(percent("0"), Some(0));
        assert_eq!(percent("101"), None);
        assert_eq!(percent("-5"), None);
        assert_eq!(percent("abc"), None);
    }
}
