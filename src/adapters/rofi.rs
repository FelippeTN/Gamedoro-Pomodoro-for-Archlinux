//! Menu de controle no rofi (script mode). Diferente do GtkMenu da Waybar, ele continua aberto
//! a cada clique: o rofi chama `gamedoro rofi <texto>` de novo com a ação escolhida em `ROFI_INFO`,
//! o comando é aplicado (com ack do daemon) e a lista é redesenhada com os valores novos.
//! Só existe enquanto o menu está aberto: custo zero no resto do tempo.

use crate::adapters::config_file::ConfigFile;
use crate::adapters::local::cache_root;
use crate::app::Command;
use crate::daemon;
use crate::domain::{Pack, Phase, Settings};
use std::process::{Command as Process, Stdio};
use std::{env, fs};

const OPEN_CACHE: &str = "open-cache";
const CLOSE: &str = "close";
const ACCENT: &str = "#f38ba8";
/// Cada segmento da barra de volume vale um passo das setas.
const VOL_STEP: u32 = 5;
const VOL_BAR: u32 = 100 / VOL_STEP;

enum Row {
    /// Cabeçalho de seção: não clicável.
    Section(String),
    /// (ícone, rótulo, ação): um comando do daemon, OPEN_CACHE ou CLOSE.
    Item(&'static str, String, &'static str),
}

/// Ponto de entrada de `gamedoro rofi`: imprime o protocolo do rofi no stdout.
pub fn run() {
    let mut error = None;
    // ROFI_RETV=1: o usuário escolheu um item; ROFI_INFO traz a ação que nós mesmos pusemos.
    // 10/11: kb-custom-1/2 (← / → no gamedoro.rasi) ajustam o volume.
    let action = match env::var("ROFI_RETV").as_deref() {
        Ok("1") => Some(env::var("ROFI_INFO").unwrap_or_default()),
        Ok("10") => Some(format!("vol -{VOL_STEP}")),
        Ok("11") => Some(format!("vol +{VOL_STEP}")),
        _ => None,
    };
    if let Some(info) = action {
        match info.as_str() {
            OPEN_CACHE => return open_cache(), // abre o gerenciador de arquivos e fecha o menu
            cmd if Command::parse(cmd).is_ok() => {
                if daemon::send(cmd).is_err() {
                    error = Some("o Gamedoro não está rodando: recarregue a waybar");
                }
            }
            _ => return, // "Fechar" (ou algo inesperado): sem linhas, o rofi fecha
        }
    }
    print!("{}", render(&ConfigFile::load().settings, error));
}

fn open_cache() {
    let dir = cache_root();
    let _ = fs::create_dir_all(&dir);
    // stdout/stderr nulos: o rofi espera o stdout do script fechar antes de continuar
    let _ = Process::new("xdg-open").arg(&dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
}

fn section(title: &str, value: Option<String>) -> Row {
    let value = value.map(|v| format!("   <span foreground='{ACCENT}'><b>{v}</b></span>")).unwrap_or_default();
    Row::Section(format!("<span size='small' alpha='55%'><b>{title}</b></span>{value}"))
}

fn item(icon: &'static str, label: &str, action: &'static str) -> Row {
    Row::Item(icon, label.to_string(), action)
}

fn rows(s: &Settings) -> Vec<Row> {
    let (focus, brk) = (s.durations.minutes(Phase::Focus), s.durations.minutes(Phase::Break));
    let pack = |p: Pack, action| {
        let mark = if p == s.pack { "\u{f0133}" } else { "\u{f0130}" }; // círculo cheio / vazio
        item(mark, p.title(), action)
    };
    vec![
        section("FOCO", Some(format!("{focus} min"))),
        item("\u{f0415}", "+5 min", "focus +5"),
        item("\u{f0374}", "−5 min", "focus -5"),
        item("\u{f0415}", "+1 min", "focus +1"),
        item("\u{f0374}", "−1 min", "focus -1"),
        section("DESCANSO", Some(format!("{brk} min"))),
        item("\u{f0415}", "+1 min", "break +1"),
        item("\u{f0374}", "−1 min", "break -1"),
        section("TRILHA", Some(s.pack.title().to_string())),
        pack(Pack::Minecraft, "music pack minecraft"),
        pack(Pack::Witcher3, "music pack witcher3"),
        pack(Pack::TheLastOfUs, "music pack tlou"),
        section("MÚSICA", None),
        item("\u{f040e}", "Liga / desliga", "music toggle"),
        item("\u{f04ad}", "Próxima faixa", "music next"),
        item("\u{f04ae}", "Faixa anterior", "music prev"),
        item("\u{f024b}", "Abrir pasta das músicas", OPEN_CACHE),
        section("VOLUME  ·  ← →", Some(format!("{}%", s.volume.percent()))),
        volume(s.volume.percent()),
        section("TIMER", None),
        item("\u{f04ac}", "Pular fase", "skip"),
        item("\u{f0453}", "Reiniciar", "reset"),
        item("\u{f0156}", "Fechar", CLOSE),
    ]
}

/// Barra clicável: cada clique sobe 10% e, no máximo, volta ao mudo (nunca pula para o alto).
fn volume(v: u32) -> Row {
    let icon = match v {
        0 => "\u{f075f}",      // mudo
        1..=33 => "\u{f057f}", // baixo
        34..=66 => "\u{f0580}", // médio
        _ => "\u{f057e}",      // alto
    };
    let filled = (v / VOL_STEP).min(VOL_BAR) as usize;
    let bar = format!(
        "<span foreground='{ACCENT}'>{}</span><span alpha='30%'>{}</span>",
        "━".repeat(filled),
        "━".repeat(VOL_BAR as usize - filled)
    );
    Row::Item(icon, bar, if v >= 100 { "vol -100" } else { "vol +10" })
}

fn render(s: &Settings, error: Option<&str>) -> String {
    let (focus, brk) = (s.durations.minutes(Phase::Focus), s.durations.minutes(Phase::Break));
    let header = match error {
        Some(e) => format!("<span foreground='{ACCENT}'>{e}</span>"),
        // `&#10;`: um \n literal encerraria a opção no protocolo do rofi e viraria uma linha do menu
        None => format!(
            "<span foreground='{ACCENT}'><b>\u{f051b}  Gamedoro</b></span>&#10;<span alpha='60%'>ciclo de {} min  ·  {}</span>",
            focus + brk,
            s.pack.title()
        ),
    };
    let rows = rows(s);
    let active = rows
        .iter()
        .position(|r| matches!(r, Row::Item(_, label, _) if label == s.pack.title()))
        .unwrap_or(0);
    let mut out = format!(
        "\0prompt\x1fGamedoro\n\0message\x1f{header}\n\0markup-rows\x1ftrue\n\0no-custom\x1ftrue\n\0keep-selection\x1ftrue\n\0active\x1f{active}\n"
    );
    for row in &rows {
        out += &match row {
            Row::Section(markup) => format!("{markup}\0nonselectable\x1ftrue\n"),
            Row::Item(icon, label, action) => format!("   {icon}   {label}\0info\x1f{action}\n"),
        };
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{Durations, Volume};

    #[test]
    fn sections_show_values_and_actions_are_valid() {
        let s = Settings { durations: Durations::new(30, 7), pack: Pack::Witcher3, volume: Volume::new(60) };
        let out = render(&s, None);
        for needle in ["<b>FOCO</b>", "<b>30 min</b>", "<b>7 min</b>", "ciclo de 37 min  ·  The Witcher 3", "<b>60%</b>", &format!("'>{}</span><span alpha='30%'>{}<", "━".repeat(12), "━".repeat(8))] {
            assert!(out.contains(needle), "faltou {needle}");
        }
        assert!(out.contains("\0active\x1f10\n"), "Witcher: 2ª linha de pack, depois de 3 seções e 6 itens");
        // cada linha do protocolo é uma opção ou uma linha do menu: nada de \n dentro do cabeçalho
        assert_eq!(out.lines().filter(|l| !l.starts_with('\0')).count(), rows(&s).len());
        for row in rows(&s) {
            if let Row::Item(_, _, action) = row {
                assert!([OPEN_CACHE, CLOSE].contains(&action) || Command::parse(action).is_ok(), "{action}");
            }
        }
        assert!(render(&s, Some("erro")).contains("erro"));
        assert!(matches!(volume(100), Row::Item(_, _, "vol -100")), "no máximo, o clique volta ao mudo");
        assert!(matches!(volume(0), Row::Item("\u{f075f}", _, "vol +10")));
    }
}
