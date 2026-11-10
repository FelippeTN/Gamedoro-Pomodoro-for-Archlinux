//! Menu de controle do Gamedoro (clique direito no timer): janela GTK3 nativa em layer-shell, com
//! o volume num slider arrastável. Substitui o rofi, que não tem slider. Continua aberto a cada
//! clique, fecha com Esc, clicando fora ou em "Fechar", e só existe enquanto está aberto.
//! Adapter de apresentação: lê o estado do config e manda comandos ao daemon pelo socket.

use gamedoro::adapters::config_file::ConfigFile;
use gamedoro::adapters::local::cache_root;
use gamedoro::daemon;
use gamedoro::domain::{Pack, Phase, Settings};
use std::cell::Cell;
use std::ffi::{c_char, c_int, c_uint, c_void, CString};
use std::process::{Command as Process, Stdio};
use std::ptr::{null, null_mut};

type W = *mut c_void;

const FOCUS: [(&str, &str); 4] = [("−5", "focus -5"), ("−1", "focus -1"), ("+1", "focus +1"), ("+5", "focus +5")];
const BREAK: [(&str, &str); 4] = [("−5", "break -5"), ("−1", "break -1"), ("+1", "break +1"), ("+5", "break +5")];
const MUSIC: [(&str, &str); 3] = [("\u{f040e}  Liga / desliga", "music toggle"), ("\u{f04ae}", "music prev"), ("\u{f04ad}", "music next")];
const TIMER: [(&str, &str); 2] = [("\u{f04ac}  Pular fase", "skip"), ("\u{f0453}  Reiniciar", "reset")];
const OPEN_CACHE: &str = "open-cache";
const CLOSE: &str = "close";
const ESCAPE: c_uint = 0xff1b;

const CSS: &str = r#"
#gamedoro-overlay { background: transparent; }
#card { background: #1e1e2e; color: #cdd6f4; border: 1px solid alpha(#f38ba8, 0.35); border-radius: 12px;
        padding: 12px 14px; margin: 10px 0 0 10px; font-family: "JetBrainsMono Nerd Font"; font-size: 10pt; }
.title { color: #f38ba8; font-weight: bold; }
.muted { color: alpha(#cdd6f4, 0.6); }
.section { color: alpha(#cdd6f4, 0.55); font-size: 8pt; font-weight: bold; margin-top: 10px; }
.value { color: #f38ba8; font-weight: bold; margin-top: 10px; }
.error { color: #f38ba8; }
#card button { background: transparent; background-image: none; color: #cdd6f4; border: none; box-shadow: none;
               text-shadow: none; border-radius: 8px; padding: 3px 10px; min-height: 0; }
#card button:hover { background: #313244; }
#card button.active { color: #f38ba8; }
#card separator { background: #45475a; min-height: 1px; margin: 8px 0 0 0; }
#card scale trough { min-height: 4px; border-radius: 4px; background: alpha(#cdd6f4, 0.2); border: none; }
#card scale highlight { background: #f38ba8; border-radius: 4px; border: none; }
#card scale slider { min-width: 14px; min-height: 14px; margin: -6px; border-radius: 50%; background: #f38ba8;
                     border: none; box-shadow: none; }
"#;

#[link(name = "gtk-3")]
extern "C" {
    fn gtk_init(argc: *mut c_int, argv: *mut c_void);
    fn gtk_main();
    fn gtk_main_quit();
    fn gtk_window_new(kind: c_int) -> W;
    fn gtk_widget_show_all(w: W);
    fn gtk_widget_set_name(w: W, name: *const c_char);
    fn gtk_widget_set_halign(w: W, align: c_int);
    fn gtk_widget_set_valign(w: W, align: c_int);
    fn gtk_widget_set_hexpand(w: W, expand: c_int);
    fn gtk_widget_set_app_paintable(w: W, paintable: c_int);
    fn gtk_widget_set_visual(w: W, visual: *mut c_void);
    fn gtk_widget_get_screen(w: W) -> *mut c_void;
    fn gtk_widget_add_events(w: W, events: c_int);
    fn gtk_widget_get_style_context(w: W) -> *mut c_void;
    fn gtk_style_context_add_class(ctx: *mut c_void, class: *const c_char);
    fn gtk_style_context_remove_class(ctx: *mut c_void, class: *const c_char);
    fn gtk_style_context_add_provider_for_screen(screen: *mut c_void, provider: *mut c_void, priority: c_uint);
    fn gtk_css_provider_new() -> *mut c_void;
    fn gtk_css_provider_load_from_data(p: *mut c_void, data: *const c_char, len: isize, err: *mut c_void) -> c_int;
    fn gtk_container_add(c: W, w: W);
    fn gtk_box_new(orientation: c_int, spacing: c_int) -> W;
    fn gtk_box_pack_start(b: W, w: W, expand: c_int, fill: c_int, padding: c_uint);
    fn gtk_label_new(text: *const c_char) -> W;
    fn gtk_label_set_markup(l: W, markup: *const c_char);
    fn gtk_label_set_xalign(l: W, x: f32);
    fn gtk_button_new_with_label(text: *const c_char) -> W;
    fn gtk_button_set_label(b: W, text: *const c_char);
    fn gtk_bin_get_child(b: W) -> W;
    fn gtk_separator_new(orientation: c_int) -> W;
    fn gtk_event_box_new() -> W;
    fn gtk_scale_new_with_range(orientation: c_int, min: f64, max: f64, step: f64) -> W;
    fn gtk_scale_set_draw_value(s: W, draw: c_int);
    fn gtk_range_set_value(r: W, v: f64);
    fn gtk_range_get_value(r: W) -> f64;
}

#[link(name = "gdk-3")]
extern "C" {
    fn gdk_screen_get_rgba_visual(screen: *mut c_void) -> *mut c_void;
    fn gdk_event_get_keyval(event: *mut c_void, keyval: *mut c_uint) -> c_int;
}

#[link(name = "gobject-2.0")]
extern "C" {
    fn g_signal_connect_data(i: W, signal: *const c_char, h: *const c_void, d: *mut c_void, destroy: *const c_void, flags: c_int) -> u64;
}

#[link(name = "gtk-layer-shell")]
extern "C" {
    fn gtk_layer_init_for_window(w: W);
    fn gtk_layer_set_namespace(w: W, ns: *const c_char);
    fn gtk_layer_set_layer(w: W, layer: c_int);
    fn gtk_layer_set_anchor(w: W, edge: c_int, anchor: c_int);
    fn gtk_layer_set_keyboard_mode(w: W, mode: c_int);
}

const VERTICAL: c_int = 1;
const HORIZONTAL: c_int = 0;
const ALIGN_START: c_int = 1;
const LAYER_OVERLAY: c_int = 3;
const KEYBOARD_EXCLUSIVE: c_int = 1;
const BUTTON_PRESS_MASK: c_int = 1 << 8;
const KEY_PRESS_MASK: c_int = 1 << 10;
const CSS_PRIORITY_APPLICATION: c_uint = 600;

fn cs(s: &str) -> CString {
    CString::new(s).unwrap_or_default()
}

/// Rótulos e controles que mudam a cada ação.
struct Menu {
    subtitle: W,
    focus: W,
    brk: W,
    pack: W,
    volume: W,
    packs: [W; 3],
    scale: W,
    /// true enquanto o próprio menu ajusta o slider (não é o usuário arrastando).
    syncing: Cell<bool>,
}

struct Click {
    menu: &'static Menu,
    action: &'static str,
}

fn main() {
    unsafe {
        gtk_init(null_mut(), null_mut());
        let provider = gtk_css_provider_new();
        gtk_css_provider_load_from_data(provider, cs(CSS).as_ptr(), -1, null_mut());

        let win = gtk_window_new(0);
        gtk_layer_init_for_window(win);
        gtk_layer_set_namespace(win, c"gamedoro-menu".as_ptr());
        gtk_layer_set_layer(win, LAYER_OVERLAY);
        for edge in 0..4 {
            gtk_layer_set_anchor(win, edge, 1); // tela inteira e transparente: clicar fora do cartão fecha
        }
        gtk_layer_set_keyboard_mode(win, KEYBOARD_EXCLUSIVE);
        let screen = gtk_widget_get_screen(win);
        gtk_style_context_add_provider_for_screen(screen, provider, CSS_PRIORITY_APPLICATION);
        let visual = gdk_screen_get_rgba_visual(screen);
        if !visual.is_null() {
            gtk_widget_set_visual(win, visual);
        }
        gtk_widget_set_app_paintable(win, 1);
        gtk_widget_set_name(win, c"gamedoro-overlay".as_ptr());
        gtk_widget_add_events(win, BUTTON_PRESS_MASK | KEY_PRESS_MASK);
        connect(win, c"button-press-event", on_outside as *const c_void, null_mut());
        connect(win, c"key-press-event", on_key as *const c_void, null_mut());
        connect(win, c"destroy", gtk_main_quit as *const c_void, null_mut());

        // o EventBox engole cliques dentro do cartão (para não contarem como "clicar fora")
        let shield = gtk_event_box_new();
        gtk_widget_set_halign(shield, ALIGN_START);
        gtk_widget_set_valign(shield, ALIGN_START);
        connect(shield, c"button-press-event", stop as *const c_void, null_mut());
        let card = gtk_box_new(VERTICAL, 2);
        gtk_widget_set_name(card, c"card".as_ptr());
        gtk_container_add(shield, card);
        gtk_container_add(win, shield);

        let menu = build(card);
        refresh(menu, None);
        gtk_widget_show_all(win);
        gtk_main();
    }
}

unsafe fn build(card: W) -> &'static Menu {
    add(card, label("\u{f051b}  Gamedoro", &["title"]));
    let subtitle = label("", &["muted"]);
    add(card, subtitle);
    add(card, gtk_separator_new(HORIZONTAL));

    let (focus, focus_row) = section("FOCO");
    let (brk, break_row) = section("DESCANSO");
    let (pack, pack_row) = section("TRILHA");
    let (volume, volume_row) = section("VOLUME");
    let scale = gtk_scale_new_with_range(HORIZONTAL, 0.0, 100.0, 1.0);
    gtk_scale_set_draw_value(scale, 0);
    gtk_widget_set_hexpand(scale, 1);
    let packs = Pack::ALL.map(|p| list_button(p.title()));

    let menu: &'static Menu = Box::leak(Box::new(Menu {
        subtitle,
        focus,
        brk,
        pack,
        volume,
        packs,
        scale,
        syncing: Cell::new(false),
    }));

    add(card, focus_row);
    add(card, button_row(menu, &FOCUS));
    add(card, break_row);
    add(card, button_row(menu, &BREAK));
    add(card, pack_row);
    for (b, p) in packs.iter().zip(Pack::ALL) {
        on_click(*b, menu, Box::leak(format!("music pack {}", p.id()).into_boxed_str()));
        add(card, *b);
    }
    add(card, section("MÚSICA").1);
    add(card, button_row(menu, &MUSIC));
    let open = list_button("\u{f024b}  Abrir pasta das músicas");
    on_click(open, menu, OPEN_CACHE);
    add(card, open);
    add(card, volume_row);
    let slider_row = gtk_box_new(HORIZONTAL, 8);
    add(slider_row, label("\u{f057e}", &["muted"]));
    gtk_box_pack_start(slider_row, scale, 1, 1, 0);
    add(card, slider_row);
    connect(scale, c"value-changed", on_volume as *const c_void, menu as *const Menu as *mut c_void);
    add(card, section("TIMER").1);
    add(card, button_row(menu, &TIMER));
    let close = list_button("\u{f0156}  Fechar");
    on_click(close, menu, CLOSE);
    add(card, close);
    menu
}

/// Relê o estado salvo e atualiza rótulos, pack ativo e slider.
unsafe fn refresh(menu: &Menu, error: Option<&str>) {
    let s = ConfigFile::load().settings;
    set_markup(menu.subtitle, &error.map_or_else(|| subtitle(&s), |e| format!("<span class='error'>{e}</span>")));
    set_markup(menu.focus, &format!("{} min", s.durations.minutes(Phase::Focus)));
    set_markup(menu.brk, &format!("{} min", s.durations.minutes(Phase::Break)));
    set_markup(menu.pack, s.pack.title());
    set_markup(menu.volume, &format!("{}%", s.volume.percent()));
    for (b, p) in menu.packs.iter().zip(Pack::ALL) {
        gtk_button_set_label(*b, cs(&pack_label(p, p == s.pack)).as_ptr());
        gtk_label_set_xalign(gtk_bin_get_child(*b), 0.0);
        let ctx = gtk_widget_get_style_context(*b);
        if p == s.pack {
            gtk_style_context_add_class(ctx, c"active".as_ptr());
        } else {
            gtk_style_context_remove_class(ctx, c"active".as_ptr());
        }
    }
    menu.syncing.set(true);
    gtk_range_set_value(menu.scale, s.volume.percent() as f64);
    menu.syncing.set(false);
}

fn subtitle(s: &Settings) -> String {
    let cycle = s.durations.minutes(Phase::Focus) + s.durations.minutes(Phase::Break);
    format!("ciclo de {cycle} min  ·  {}", s.pack.title())
}

fn pack_label(p: Pack, active: bool) -> String {
    format!("{}  {}", if active { "\u{f0133}" } else { "\u{f0130}" }, p.title())
}

/// Aplica a ação no daemon (com ack) e redesenha; "Fechar" e "Abrir pasta" fecham o menu.
unsafe fn run(menu: &Menu, action: &str) {
    match action {
        CLOSE => gtk_main_quit(),
        OPEN_CACHE => {
            let dir = cache_root();
            let _ = std::fs::create_dir_all(&dir);
            let _ = Process::new("xdg-open").arg(&dir).stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null()).spawn();
            gtk_main_quit();
        }
        cmd => {
            let error = daemon::send(cmd).err().map(|_| "o Gamedoro não está rodando: recarregue a waybar");
            refresh(menu, error);
        }
    }
}

unsafe extern "C" fn on_button(_b: W, data: *mut c_void) {
    let click = &*(data as *const Click);
    run(click.menu, click.action);
}

unsafe extern "C" fn on_volume(range: W, data: *mut c_void) {
    let menu = &*(data as *const Menu);
    if !menu.syncing.get() {
        let v = gtk_range_get_value(range).round() as u32;
        run(menu, &format!("vol {v}"));
    }
}

unsafe extern "C" fn on_key(_w: W, event: *mut c_void, _d: *mut c_void) -> c_int {
    let mut key: c_uint = 0;
    if gdk_event_get_keyval(event, &mut key) != 0 && key == ESCAPE {
        gtk_main_quit();
        return 1;
    }
    0
}

unsafe extern "C" fn on_outside(_w: W, _e: *mut c_void, _d: *mut c_void) -> c_int {
    gtk_main_quit();
    1
}

unsafe extern "C" fn stop(_w: W, _e: *mut c_void, _d: *mut c_void) -> c_int {
    1
}

unsafe fn connect(w: W, signal: &std::ffi::CStr, handler: *const c_void, data: *mut c_void) {
    g_signal_connect_data(w, signal.as_ptr(), handler, data, null(), 0);
}

unsafe fn on_click(button: W, menu: &'static Menu, action: &'static str) {
    let click = Box::leak(Box::new(Click { menu, action })) as *mut Click as *mut c_void;
    connect(button, c"clicked", on_button as *const c_void, click);
}

unsafe fn add(container: W, child: W) {
    gtk_box_pack_start(container, child, 0, 0, 0);
}

unsafe fn set_markup(l: W, markup: &str) {
    gtk_label_set_markup(l, cs(markup).as_ptr());
}

unsafe fn label(markup: &str, classes: &[&str]) -> W {
    let l = gtk_label_new(null());
    set_markup(l, markup);
    gtk_label_set_xalign(l, 0.0);
    for c in classes {
        gtk_style_context_add_class(gtk_widget_get_style_context(l), cs(c).as_ptr());
    }
    l
}

/// Título da seção + valor atual ao lado (o valor é atualizado no `refresh`).
unsafe fn section(title: &str) -> (W, W) {
    let row = gtk_box_new(HORIZONTAL, 10);
    add(row, label(title, &["section"]));
    let value = label("", &["value"]);
    add(row, value);
    (value, row)
}

unsafe fn list_button(text: &str) -> W {
    let b = gtk_button_new_with_label(cs(text).as_ptr());
    gtk_label_set_xalign(gtk_bin_get_child(b), 0.0);
    b
}

unsafe fn button_row(menu: &'static Menu, items: &[(&str, &'static str)]) -> W {
    let row = gtk_box_new(HORIZONTAL, 4);
    for (text, action) in items {
        let b = gtk_button_new_with_label(cs(text).as_ptr());
        on_click(b, menu, action);
        add(row, b);
    }
    row
}

#[cfg(test)]
mod tests {
    use super::*;
    use gamedoro::app::Command;
    use gamedoro::domain::{Durations, Volume};

    #[test]
    fn every_button_sends_a_valid_command() {
        let packs = Pack::ALL.map(|p| format!("music pack {}", p.id()));
        let actions = FOCUS.iter().chain(&BREAK).chain(&MUSIC).chain(&TIMER).map(|(_, a)| *a).chain(packs.iter().map(String::as_str));
        for a in actions {
            assert!(Command::parse(a).is_ok(), "{a}");
        }
        assert!(Command::parse("vol 37").is_ok(), "comando do slider");
    }

    #[test]
    fn labels() {
        let s = Settings { durations: Durations::new(30, 7), pack: Pack::Witcher3, volume: Volume::new(60) };
        assert_eq!(subtitle(&s), "ciclo de 37 min  ·  The Witcher 3");
        assert!(pack_label(Pack::Minecraft, true).ends_with("Minecraft") && pack_label(Pack::Minecraft, true).starts_with('\u{f0133}'));
    }
}
