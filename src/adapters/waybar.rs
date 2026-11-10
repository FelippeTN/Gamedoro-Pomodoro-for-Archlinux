//! Presenter: transforma o estado do Pomodoro na linha JSON do módulo custom da Waybar.
//! O tooltip é markup Pango; a fonte da barra (JetBrainsMono Nerd) é monoespaçada, então as colunas alinham.

use crate::app::{MusicStatus, Pomodoro};
use crate::domain::Phase;

const ICON_FOCUS: &str = "\u{f051b}"; // nf-md-timer_outline
const ICON_BREAK: &str = "\u{f0f4}"; // nf-fa-coffee
const ICON_PAUSED: &str = "\u{f04c}"; // nf-fa-pause
const ICON_MUSIC: &str = "\u{f1bc}"; // nf-fa-spotify
const ACCENT_FOCUS: &str = "#f38ba8"; // mesmas cores do gamedoro.css
const ACCENT_BREAK: &str = "#a6e3a1";
const WIDTH: usize = 36; // colunas (fonte mono) de TODAS as linhas: o tooltip fica com largura fixa
const BAR: usize = 30;
/// Ícones Nerd Font ocupam ~1 coluna (medido renderizando com a fonte da Waybar).
const ICON_COLS: usize = 1;

/// `now`: segundos desde a meia-noite local (injetado para manter o presenter puro e testável).
pub fn render(app: &Pomodoro, now: u32) -> String {
    let t = app.timer();
    let phase = t.phase();
    let (icon, class, accent) = match phase {
        Phase::Focus => (ICON_FOCUS, "focus", ACCENT_FOCUS),
        Phase::Break => (ICON_BREAK, "break", ACCENT_BREAK),
    };
    let alert = app.is_alerting();
    let bar_icon = if t.is_running() || alert { icon } else { ICON_PAUSED };
    let state = if t.is_running() { "running" } else { "paused" };
    let alert_class = if alert { ",\"alert\"" } else { "" };
    let left = t.seconds_left();
    format!(
        "{{\"text\":{},\"alt\":\"{class}\",\"tooltip\":{},\"class\":[\"{class}\",\"{state}\"{alert_class}],\"percentage\":{}}}",
        json_str(&format!("{bar_icon} {}", mmss(left))),
        json_str(&tooltip(app, now, icon, accent)),
        t.progress_percent(),
    )
}

/// GTK alinha o tooltip à esquerda e o Pango não tem "center": cada linha recebe a largura
/// visível (em colunas) e é centralizada com espaços dentro de WIDTH.
fn tooltip(app: &Pomodoro, now: u32, icon: &str, accent: &str) -> String {
    let t = app.timer();
    let d = t.durations();
    let phase = t.phase();
    let next = phase.other();
    let left = t.seconds_left();
    let pct = t.progress_percent();
    let status = if app.is_alerting() {
        "nova fase"
    } else if t.is_running() {
        "em andamento"
    } else {
        "pausado"
    };
    let (focus, brk) = (d.minutes(Phase::Focus), d.minutes(Phase::Break));

    let title = format!("{}  ·  {status}", phase_name(phase).to_uppercase());
    let total = format!("  /  {}", mmss(d.minutes(phase) * 60));
    let eta = if t.is_running() {
        format!("{pct}% concluído · termina às {}", hhmm(now + left))
    } else {
        format!("{pct}% concluído · pausado")
    };
    let after = format!("depois: {} de {} min", phase_name(next).to_lowercase(), d.minutes(next));
    let pack = app.pack().title();
    let music = match app.music() {
        MusicStatus::Unavailable => "spotifyd off · veja o README".to_string(),
        MusicStatus::Off => "música desligada".to_string(),
        MusicStatus::Playing(track) => truncate(track, WIDTH),
    };
    let hints = ["clique play/pause · meio pular", "scroll foco ±1 · direito menu"];

    let mut lines = vec![
        center(
            ICON_COLS + 2 + cols(&title),
            format!("<span foreground='{accent}'><b>{icon}  {}</b></span>  {}", phase_name(phase).to_uppercase(), muted(&format!("·  {status}"))),
        ),
        String::new(),
        // 200%: cada caractere do timer ocupa exatamente 2 colunas
        center(
            2 * 5 + cols(&total),
            format!("<span size='200%' foreground='{accent}'><b>{}</b></span>{}", mmss(left), muted(&total)),
        ),
        center(BAR, bar(pct, accent)),
        center(cols(&eta), muted(&eta)),
        center(cols(&after), muted(&after)),
        String::new(),
        section("Sessão"),
        row("Foco", &format!("{focus} min")),
        row("Descanso", &format!("{brk} min")),
        row("Ciclo completo", &format!("{} min", focus + brk)),
        row("Pomodoros concluídos", &t.completed().to_string()),
        row("Tempo focado", &duration(t.focused_seconds())),
        String::new(),
        section("Música"),
        center(ICON_COLS + 2 + cols(pack), format!("{ICON_MUSIC}  <b>{pack}</b>")),
        center(cols(&music), muted(&pango(&music))),
        String::new(),
        muted(&"─".repeat(WIDTH)),
    ];
    lines.extend(hints.iter().map(|h| center(cols(h), muted(h))));
    lines.join("\n")
}

fn cols(s: &str) -> usize {
    s.chars().count()
}

fn center(width: usize, markup: String) -> String {
    format!("{}{markup}", " ".repeat(WIDTH.saturating_sub(width) / 2))
}

fn truncate(s: &str, max: usize) -> String {
    if cols(s) <= max {
        return s.to_string();
    }
    s.chars().take(max - 1).chain(['…']).collect()
}

fn phase_name(p: Phase) -> &'static str {
    match p {
        Phase::Focus => "Foco",
        Phase::Break => "Descanso",
    }
}

fn muted(s: &str) -> String {
    format!("<span alpha='55%'>{s}</span>")
}

fn section(title: &str) -> String {
    let title = title.to_uppercase();
    let side = WIDTH.saturating_sub(cols(&title) + 2);
    muted(&format!("{} <b>{title}</b> {}", "─".repeat(side / 2), "─".repeat(side - side / 2)))
}

fn row(label: &str, value: &str) -> String {
    let pad = WIDTH.saturating_sub(cols(label) + cols(value));
    format!("{label}{}<b>{value}</b>", " ".repeat(pad))
}

fn bar(pct: u32, accent: &str) -> String {
    let filled = (pct.min(100) as usize * BAR).div_ceil(100);
    format!(
        "<span foreground='{accent}'>{}</span><span alpha='30%'>{}</span>",
        "━".repeat(filled),
        "━".repeat(BAR - filled)
    )
}

fn mmss(secs: u32) -> String {
    format!("{:02}:{:02}", secs / 60, secs % 60)
}

fn hhmm(secs_of_day: u32) -> String {
    let s = secs_of_day % 86_400;
    format!("{:02}:{:02}", s / 3600, s % 3600 / 60)
}

fn duration(secs: u32) -> String {
    match (secs / 3600, secs % 3600 / 60) {
        (0, m) => format!("{m} min"),
        (h, m) => format!("{h}h{m:02}"),
    }
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
    use crate::app::testing::app;
    use crate::app::Command;

    #[test]
    fn escaping() {
        assert_eq!(json_str("a\"b\\c\nd\u{1}"), r#""a\"b\\c\nd\u0001""#);
        assert_eq!(pango("Tom & <Jerry>"), "Tom &amp; &lt;Jerry&gt;");
    }

    #[test]
    fn formatting() {
        assert_eq!((mmss(1500), hhmm(14 * 3600 + 32 * 60), hhmm(86_400 + 60)), ("25:00".into(), "14:32".into(), "00:01".into()));
        assert_eq!((duration(1500), duration(4500)), ("25 min".into(), "1h15".into()));
        assert_eq!(cols(&row("Foco", "25 min")), WIDTH + "<b></b>".len());
        assert_eq!(cols(&section("Sessão").replace("<span alpha='55%'>", "").replace("</span>", "").replace("<b>", "").replace("</b>", "")), WIDTH);
        assert_eq!(truncate("abcdef", 4), "abc…");
        assert_eq!(center(30, "x".into()), "   x");
    }

    #[test]
    fn tooltip_shows_totals_eta_and_break() {
        let (mut p, _) = app(25); // foco 25, descanso 1
        p.execute(Command::Toggle);
        let tip = tooltip(&p, 14 * 3600, ICON_FOCUS, ACCENT_FOCUS);
        for needle in ["25:00", "/  25:00", "0% concluído · termina às 14:25", "depois: descanso de 1 min", "<b>1 min</b>", "<b>26 min</b>", "C418 – Sweden"] {
            assert!(tip.contains(needle), "faltou {needle:?} em:\n{tip}");
        }
        let line = render(&p, 0);
        assert!(line.contains(r#""class":["focus","running"]"#));
    }
}
