//! Portas: o que a aplicação precisa do mundo externo (inversão de dependência).

use crate::domain::{Pack, Phase, Settings};

pub trait MusicPlayer {
    /// Carrega o pack e começa a tocar. Retorna false se o player não está disponível.
    fn load(&mut self, pack: Pack) -> bool;
    fn play(&mut self);
    fn pause(&mut self);
    fn next(&mut self);
    fn previous(&mut self);
    fn change_volume(&mut self, percent: i64);
    /// "Artista – Faixa", ou None se o player não está disponível.
    fn now_playing(&mut self) -> Option<String>;
}

pub trait Notifier {
    fn phase_started(&self, phase: Phase, minutes: u32);
}

pub trait SettingsStore {
    fn save(&self, settings: &Settings);
}
