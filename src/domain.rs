//! Domínio puro do pomodoro: regras de tempo e fases. Sem IO, sem relógio, sem música.

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Phase {
    Focus,
    Break,
}

impl Phase {
    pub fn other(self) -> Phase {
        match self {
            Phase::Focus => Phase::Break,
            Phase::Break => Phase::Focus,
        }
    }
}

/// Trilha sonora escolhida. O domínio só conhece o pack; como tocá-lo é problema do adapter.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum Pack {
    #[default]
    Minecraft,
    Witcher3,
    TheLastOfUs,
}

impl Pack {
    pub const ALL: [Pack; 3] = [Pack::Minecraft, Pack::Witcher3, Pack::TheLastOfUs];

    pub fn id(self) -> &'static str {
        match self {
            Pack::Minecraft => "minecraft",
            Pack::Witcher3 => "witcher3",
            Pack::TheLastOfUs => "tlou",
        }
    }

    pub fn title(self) -> &'static str {
        match self {
            Pack::Minecraft => "Minecraft",
            Pack::Witcher3 => "The Witcher 3",
            Pack::TheLastOfUs => "The Last of Us",
        }
    }

    pub fn from_id(id: &str) -> Option<Pack> {
        Pack::ALL.into_iter().find(|p| p.id() == id)
    }

    pub fn index(self) -> usize {
        self as usize
    }

    pub fn next(self) -> Pack {
        Pack::ALL[(self.index() + 1) % Pack::ALL.len()]
    }
}

/// Value object: durações em minutos, sempre dentro de [MIN, MAX].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Durations {
    focus: u32,
    brk: u32,
}

impl Durations {
    pub const MIN: u32 = 1;
    pub const MAX: u32 = 180;

    pub fn new(focus: i64, brk: i64) -> Durations {
        Durations { focus: Self::clamp(focus), brk: Self::clamp(brk) }
    }

    fn clamp(m: i64) -> u32 {
        m.clamp(Self::MIN as i64, Self::MAX as i64) as u32
    }

    pub fn minutes(self, p: Phase) -> u32 {
        match p {
            Phase::Focus => self.focus,
            Phase::Break => self.brk,
        }
    }

    pub fn with(self, p: Phase, minutes: i64) -> Durations {
        match p {
            Phase::Focus => Durations::new(minutes, self.brk as i64),
            Phase::Break => Durations::new(self.focus as i64, minutes),
        }
    }
}

impl Default for Durations {
    fn default() -> Self {
        Durations { focus: 25, brk: 5 }
    }
}

/// Value object: volume da música em porcento, sempre dentro de [0, 100].
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct Volume(u32);

impl Volume {
    pub fn new(percent: i64) -> Volume {
        Volume(percent.clamp(0, 100) as u32)
    }

    pub fn percent(self) -> u32 {
        self.0
    }

    pub fn adjusted(self, delta: i64) -> Volume {
        Volume::new(self.0 as i64 + delta)
    }
}

impl Default for Volume {
    fn default() -> Self {
        Volume(60)
    }
}

/// Preferências persistidas do usuário.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Settings {
    pub durations: Durations,
    pub pack: Pack,
    pub volume: Volume,
}

/// Aggregate root: o ciclo foco/descanso. Avança por `tick()` de 1 s vindo de fora.
#[derive(Debug)]
pub struct Timer {
    durations: Durations,
    phase: Phase,
    left: u32,
    running: bool,
    completed: u32,
    focused: u32,
}

impl Timer {
    pub fn new(durations: Durations) -> Timer {
        Timer { durations, phase: Phase::Focus, left: durations.minutes(Phase::Focus) * 60, running: false, completed: 0, focused: 0 }
    }

    pub fn phase(&self) -> Phase {
        self.phase
    }
    pub fn seconds_left(&self) -> u32 {
        self.left
    }
    pub fn is_running(&self) -> bool {
        self.running
    }
    pub fn completed(&self) -> u32 {
        self.completed
    }
    /// Segundos efetivamente focados (rodando em foco) nesta sessão.
    pub fn focused_seconds(&self) -> u32 {
        self.focused
    }
    pub fn durations(&self) -> Durations {
        self.durations
    }

    pub fn progress_percent(&self) -> u32 {
        let total = self.durations.minutes(self.phase) * 60;
        100 - self.left.min(total) * 100 / total
    }

    pub fn toggle(&mut self) {
        self.running = !self.running;
    }

    /// Avança 1 s. Retorna a nova fase quando a atual termina (um foco concluído conta pomodoro).
    pub fn tick(&mut self) -> Option<Phase> {
        if !self.running {
            return None;
        }
        if self.phase == Phase::Focus {
            self.focused += 1;
        }
        self.left = self.left.saturating_sub(1);
        if self.left > 0 {
            return None;
        }
        if self.phase == Phase::Focus {
            self.completed += 1;
        }
        self.skip();
        Some(self.phase)
    }

    /// Pula para a próxima fase sem contar pomodoro.
    pub fn skip(&mut self) -> Phase {
        self.phase = self.phase.other();
        self.left = self.durations.minutes(self.phase) * 60;
        self.phase
    }

    /// Volta ao início do foco, parado, mantendo a contagem.
    pub fn reset(&mut self) {
        *self = Timer { completed: self.completed, focused: self.focused, ..Timer::new(self.durations) };
    }

    /// Muda a duração de `p` em `delta` minutos; se `p` é a fase atual, o restante acompanha.
    pub fn adjust(&mut self, p: Phase, delta: i64) {
        let old = self.durations.minutes(p) as i64;
        self.durations = self.durations.with(p, old + delta);
        if self.phase == p {
            let diff = self.durations.minutes(p) as i64 - old;
            self.left = (self.left as i64 + diff * 60).max(1) as u32;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cycle_counts_only_completed_focus() {
        let mut t = Timer::new(Durations::new(1, 1));
        assert_eq!(t.tick(), None, "parado não anda");
        t.toggle();
        for _ in 0..59 {
            assert_eq!(t.tick(), None);
        }
        assert_eq!(t.tick(), Some(Phase::Break));
        assert_eq!((t.seconds_left(), t.completed(), t.focused_seconds()), (60, 1, 60));
        t.tick();
        assert_eq!(t.focused_seconds(), 60, "descanso não conta como foco");
        assert_eq!(t.skip(), Phase::Focus);
        assert_eq!(t.completed(), 1, "skip não conta");
        t.reset();
        assert!(!t.is_running());
        assert_eq!((t.completed(), t.focused_seconds()), (1, 60), "reset mantém a sessão");
    }

    #[test]
    fn adjust_clamps_and_moves_remaining() {
        let mut t = Timer::new(Durations::default());
        t.adjust(Phase::Focus, -24);
        assert_eq!((t.durations().minutes(Phase::Focus), t.seconds_left()), (1, 60));
        t.adjust(Phase::Focus, -10);
        assert_eq!(t.durations().minutes(Phase::Focus), Durations::MIN);
        t.adjust(Phase::Break, 500);
        assert_eq!(t.durations().minutes(Phase::Break), Durations::MAX);
        assert_eq!(t.seconds_left(), 60, "fase não ativa não mexe no restante");
    }

    #[test]
    fn pack_ids_roundtrip_and_cycle() {
        for p in Pack::ALL {
            assert_eq!(Pack::from_id(p.id()), Some(p));
        }
        assert_eq!(Pack::TheLastOfUs.next(), Pack::Minecraft);
        assert_eq!(Pack::from_id("zelda"), None);
    }
}
