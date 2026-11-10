//! Portas: o que a aplicação precisa do mundo externo (inversão de dependência).

use crate::domain::{Pack, Phase, Settings};

pub trait MusicPlayer {
    /// Prepara o pack (pode disparar um download). Retorna false se o player não tem como tocá-lo.
    fn load(&mut self, pack: Pack) -> bool;
    fn play(&mut self);
    fn pause(&mut self);
    fn next(&mut self);
    fn previous(&mut self);
    fn change_volume(&mut self, percent: i64);
    /// Texto da faixa/estado para a UI, ou None se o player está indisponível.
    fn now_playing(&mut self) -> Option<String>;
    /// Trabalho periódico (fim de faixa, download concluído). Chamado a cada tick de 1 s.
    fn poll(&mut self) {}
    /// true se precisa de `poll` mesmo com o timer parado (ex.: baixando).
    fn busy(&self) -> bool {
        false
    }
}

pub trait Notifier {
    fn phase_started(&self, phase: Phase, minutes: u32);
}

pub trait SettingsStore {
    fn save(&self, settings: &Settings);
}
