//! Notifier do desktop: chime via `pw-play` + notificação via `notify-send`.

use crate::domain::Phase;
use crate::ports::Notifier;
use std::process::{Command, Stdio};
use std::thread;

const CHIME: &str = "/usr/share/sounds/freedesktop/stereo/alarm-clock-elapsed.oga";

pub struct Desktop;

impl Notifier for Desktop {
    fn phase_started(&self, phase: Phase, minutes: u32) {
        let (title, body) = match phase {
            Phase::Break => ("☕ Hora do descanso", format!("{minutes} min. Música pausada.")),
            Phase::Focus => ("🎯 De volta ao foco", format!("{minutes} min. Bora!")),
        };
        // thread colhe os filhos: não bloqueia o loop e não deixa zumbis
        thread::spawn(move || {
            let chime = Command::new("pw-play").arg(CHIME).stderr(Stdio::null()).spawn();
            let _ = Command::new("notify-send")
                .args(["-a", "Gamedoro", "-i", "alarm-clock", "-u", "critical", "-t", "8000", title, &body])
                .status();
            if let Ok(mut c) = chime {
                let _ = c.wait();
            }
        });
    }
}
