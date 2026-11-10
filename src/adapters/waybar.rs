//! Presenter: transforma o estado do Pomodoro na linha JSON do módulo custom da Waybar.

use crate::app::{MusicStatus, Pomodoro};
use crate::domain::Phase;

const ICON_FOCUS: &str = "\u{f051b}"; // nf-md-timer_outline
const ICON_BREAK: &str = "\u{f0f4}"; // nf-fa-coffee
const ICON_PAUSED: &str = "\u{f04c}"; // nf-fa-pause
const ICON_MUSIC: &str = "\u{f1bc}"; // nf-fa-spotify

pub fn render(app: &Pomodoro) -> String {
    let t = app.timer();
    let (icon, phase_class, name) = match t.phase() {
        Phase::Focus => (ICON_FOCUS, "focus", "Foco"),
        Phase::Break => (ICON_BREAK, "break", "Descanso"),
    };
    let alert = app.is_alerting();
    let icon = if t.is_running() || alert { icon } else { ICON_PAUSED };
    let state = if t.is_running() { "running" } else { "paused" };
    let alert_class = if alert { ",\"alert\"" } else { "" };
    let music = match app.music() {
        MusicStatus::Unavailable => "spotifyd off".to_string(),
        MusicStatus::Off => "música desligada".to_string(),
        MusicStatus::Playing(track) => pango(track),
    };
    let d = t.durations();
    let tooltip = format!(
        "<b>{name}</b> · {}\nFoco {} min · Descanso {} min\nPomodoros nesta sessão: {}\n{ICON_MUSIC}  {}: {music}\n\n\
         <small>clique: play/pause · meio: pular · scroll: foco ±1 · direito: menu</small>",
        if t.is_running() { "rodando" } else { "pausado" },
        d.minutes(Phase::Focus),
        d.minutes(Phase::Break),
        t.completed(),
        app.pack().title(),
    );
    let left = t.seconds_left();
    format!(
        "{{\"text\":{},\"alt\":\"{phase_class}\",\"tooltip\":{},\"class\":[\"{phase_class}\",\"{state}\"{alert_class}],\"percentage\":{}}}",
        json_str(&format!("{icon} {:02}:{:02}", left / 60, left % 60)),
        json_str(&tooltip),
        t.progress_percent(),
    )
}

fn json_str(s: &str) -> String {
    let mut o = String::with_capacity(s.len() + 2);
    o.push('"');
    for c in s.chars() {
        match c {
            '"' => o.push_str("\\\""),
            '\\' => o.push_str("\\\\"),
            '\n' => o.push_str("\\n"),
            c if (c as u32) < 0x20 => o.push_str(&format!("\\u{:04x}", c as u32)),
            c => o.push(c),
        }
    }
    o.push('"');
    o
}

/// Tooltip da Waybar é markup Pango: nomes de faixa com & ou < quebrariam o render.
fn pango(s: &str) -> String {
    s.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn escaping() {
        assert_eq!(json_str("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
        assert_eq!(pango("Tom & <Jerry>"), "Tom &amp; &lt;Jerry&gt;");
    }
}
