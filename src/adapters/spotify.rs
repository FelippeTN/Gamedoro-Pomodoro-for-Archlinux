//! MusicPlayer via spotifyd, controlado por MPRIS com o `playerctl` (sem crates de D-Bus).

use crate::domain::Pack;
use crate::ports::MusicPlayer;
use std::process::{Command, Stdio};
use std::thread;
use std::time::Duration;

pub const DEFAULT_URIS: [&str; 3] = [
    "spotify:album:3Gt7rOjcZQoHCfnKl5AkK7", // C418 – Minecraft Volume Alpha
    "spotify:album:6pYNEn4tMc6gdv5fIZf5yn", // The Witcher 3: Wild Hunt OST
    "spotify:album:2GFFxj8aR2XpwIMYanOPjh", // Gustavo Santaolalla – The Last of Us
];

/// O spotifyd atualiza o metadata de forma assíncrona depois de next/open.
const METADATA_SETTLE: Duration = Duration::from_millis(500);

pub struct Spotifyd {
    uris: [String; 3],
}

impl Spotifyd {
    pub fn new(uris: [String; 3]) -> Spotifyd {
        Spotifyd { uris }
    }

    fn ctl(&self, args: &[&str]) -> Option<String> {
        let out = Command::new("playerctl")
            .args(["-p", "spotifyd"])
            .args(args)
            .stdin(Stdio::null())
            .stderr(Stdio::null())
            .output()
            .ok()?;
        out.status.success().then(|| String::from_utf8_lossy(&out.stdout).trim().to_string())
    }
}

impl MusicPlayer for Spotifyd {
    fn load(&mut self, pack: Pack) -> bool {
        let ok = self.ctl(&["open", &self.uris[pack.index()]]).is_some();
        thread::sleep(METADATA_SETTLE);
        ok
    }
    fn play(&mut self) {
        self.ctl(&["play"]);
    }
    fn pause(&mut self) {
        self.ctl(&["pause"]);
    }
    fn next(&mut self) {
        self.ctl(&["next"]);
        thread::sleep(METADATA_SETTLE);
    }
    fn previous(&mut self) {
        self.ctl(&["previous"]);
        thread::sleep(METADATA_SETTLE);
    }
    fn change_volume(&mut self, percent: i64) {
        let sign = if percent < 0 { '-' } else { '+' };
        self.ctl(&["volume", &format!("{:.2}{sign}", percent.unsigned_abs() as f64 / 100.0)]);
    }
    fn now_playing(&mut self) -> Option<String> {
        let t = self.ctl(&["metadata", "--format", "{{artist}} – {{title}}"])
            .or_else(|| self.ctl(&["status"]).map(|_| String::new()))?;
        Some(if t.trim_matches(['–', ' ']).is_empty() { "pronto".into() } else { t })
    }
}
