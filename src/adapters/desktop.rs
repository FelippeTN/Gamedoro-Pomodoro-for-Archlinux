//! Notifier do desktop: chime sintetizado com `rodio` + notificação via `notify-send`.

use crate::domain::Phase;
use crate::ports::Notifier;
use rodio::buffer::SamplesBuffer;
use rodio::{nz, DeviceSinkBuilder, Player};
use std::f32::consts::TAU;
use std::process::Command;
use std::thread;

const RATE: u32 = 44_100;
/// Arpejo de Dó maior: sobe ao voltar pro foco, desce ao entrar no descanso.
const FOCUS_NOTES: [f32; 4] = [523.25, 659.25, 783.99, 1046.50];
const BREAK_NOTES: [f32; 4] = [1046.50, 783.99, 659.25, 523.25];
const NOTE_GAP: f32 = 0.16;
const RING: f32 = 1.6;

pub struct Desktop;

impl Notifier for Desktop {
    fn phase_started(&self, phase: Phase, minutes: u32) {
        let (title, body, notes) = match phase {
            Phase::Break => ("☕ Hora do descanso", format!("{minutes} min. Música pausada."), BREAK_NOTES),
            Phase::Focus => ("🎯 De volta ao foco", format!("{minutes} min. Bora!"), FOCUS_NOTES),
        };
        // thread: não bloqueia o loop, segura o dispositivo só enquanto o chime toca e colhe o notify-send
        thread::spawn(move || {
            let notify = Command::new("notify-send")
                .args(["-a", "Gamedoro", "-i", "alarm-clock", "-u", "critical", "-t", "8000", title, &body])
                .spawn();
            if let Ok(mut sink) = DeviceSinkBuilder::open_default_sink() {
                sink.log_on_drop(false);
                let player = Player::connect_new(sink.mixer());
                player.append(SamplesBuffer::new(nz!(1), nz!(44_100), chime(&notes)));
                player.sleep_until_end();
            }
            if let Ok(mut c) = notify {
                let _ = c.wait();
            }
        });
    }
}

/// Notas tipo marimba: ataque curto, decaimento exponencial e um harmônico leve.
fn chime(notes: &[f32]) -> Vec<f32> {
    let total = NOTE_GAP * (notes.len() - 1) as f32 + RING;
    let mut out = vec![0.0f32; (total * RATE as f32) as usize];
    for (i, &f) in notes.iter().enumerate() {
        let start = (i as f32 * NOTE_GAP * RATE as f32) as usize;
        for (n, s) in out[start..].iter_mut().enumerate() {
            let t = n as f32 / RATE as f32;
            let env = (t / 0.004).min(1.0) * (-t * 3.5).exp();
            *s += env * ((TAU * f * t).sin() + 0.25 * (TAU * 4.0 * f * t).sin() * (-t * 9.0).exp());
        }
    }
    let peak = out.iter().fold(0.0f32, |m, s| m.max(s.abs()));
    out.iter_mut().for_each(|s| *s *= 0.3 / peak); // volume baixo e constante
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn chime_is_quiet_and_fades_out() {
        let s = super::chime(&super::FOCUS_NOTES);
        let peak = s.iter().fold(0.0f32, |m, x| m.max(x.abs()));
        assert!((peak - 0.3).abs() < 1e-4);
        assert!(s.last().unwrap().abs() < 0.01, "termina sem estalo");
    }
}
