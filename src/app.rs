//! Camada de aplicação: casos de uso (comandos) que orquestram o domínio e as portas.

use crate::domain::{Pack, Phase, Settings, Timer};
use crate::ports::{MusicPlayer, Notifier, SettingsStore};

/// Segundos que a waybar fica em estado de alerta após trocar de fase.
const ALERT_SECONDS: u32 = 10;
/// A cada quantos segundos de música o título da faixa é relido.
const TRACK_REFRESH_SECONDS: u32 = 15;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Command {
    Toggle,
    Skip,
    Reset,
    Adjust(Phase, i64),
    MusicToggle,
    MusicNext,
    MusicPrevious,
    MusicCycle,
    MusicPack(Pack),
    Volume(i64),
}

impl Command {
    pub fn parse(line: &str) -> Result<Command, String> {
        let mut w = line.split_whitespace();
        let num = |s: Option<&str>| s.and_then(|s| s.parse::<i64>().ok()).ok_or(format!("número inválido em '{line}'"));
        Ok(match (w.next(), w.next(), w.next()) {
            (Some("toggle"), None, _) => Command::Toggle,
            (Some("skip"), None, _) => Command::Skip,
            (Some("reset"), None, _) => Command::Reset,
            (Some("focus"), n, None) => Command::Adjust(Phase::Focus, num(n)?),
            (Some("break"), n, None) => Command::Adjust(Phase::Break, num(n)?),
            (Some("vol"), n, None) => Command::Volume(num(n)?),
            (Some("music"), Some("toggle"), None) => Command::MusicToggle,
            (Some("music"), Some("next"), None) => Command::MusicNext,
            (Some("music"), Some("prev"), None) => Command::MusicPrevious,
            (Some("music"), Some("cycle"), None) => Command::MusicCycle,
            (Some("music"), Some("pack"), Some(id)) => {
                Command::MusicPack(Pack::from_id(id).ok_or(format!("pack desconhecido: {id}"))?)
            }
            _ => return Err(format!("comando desconhecido: '{line}'")),
        })
    }
}

pub enum MusicStatus<'a> {
    Unavailable,
    Off,
    Playing(&'a str),
}

pub struct Pomodoro {
    timer: Timer,
    pack: Pack,
    music_on: bool,
    loaded: bool,
    track: Option<String>,
    alert_left: u32,
    player: Box<dyn MusicPlayer>,
    notifier: Box<dyn Notifier>,
    store: Box<dyn SettingsStore>,
}

impl Pomodoro {
    pub fn new(settings: Settings, player: Box<dyn MusicPlayer>, notifier: Box<dyn Notifier>, store: Box<dyn SettingsStore>) -> Pomodoro {
        let mut p = Pomodoro {
            timer: Timer::new(settings.durations),
            pack: settings.pack,
            music_on: true,
            loaded: false,
            track: None,
            alert_left: 0,
            player,
            notifier,
            store,
        };
        p.store.save(&p.settings()); // cria o arquivo na 1ª execução para o usuário editar
        p.track = p.player.now_playing();
        p
    }

    pub fn timer(&self) -> &Timer {
        &self.timer
    }
    pub fn pack(&self) -> Pack {
        self.pack
    }
    pub fn is_alerting(&self) -> bool {
        self.alert_left > 0
    }
    pub fn music(&self) -> MusicStatus<'_> {
        match (&self.track, self.music_on) {
            (None, _) => MusicStatus::Unavailable,
            (Some(_), false) => MusicStatus::Off,
            (Some(t), true) => MusicStatus::Playing(t),
        }
    }

    /// Precisa de tick de 1 s? Se não, o daemon pode dormir sem wakeups.
    pub fn needs_ticks(&self) -> bool {
        self.timer.is_running() || self.is_alerting()
    }

    pub fn execute(&mut self, cmd: Command) {
        match cmd {
            Command::Toggle => {
                self.timer.toggle();
                self.sync_music();
            }
            Command::Skip => {
                let phase = self.timer.skip();
                self.on_phase_started(phase);
            }
            Command::Reset => {
                self.timer.reset();
                self.alert_left = 0;
                self.sync_music();
            }
            Command::Adjust(phase, delta) => {
                self.timer.adjust(phase, delta);
                self.store.save(&self.settings());
            }
            Command::MusicToggle => {
                self.music_on = !self.music_on;
                self.sync_music();
            }
            Command::MusicNext => {
                self.player.next();
                self.track = self.player.now_playing();
            }
            Command::MusicPrevious => {
                self.player.previous();
                self.track = self.player.now_playing();
            }
            Command::MusicCycle => self.select_pack(self.pack.next()),
            Command::MusicPack(p) => self.select_pack(p),
            Command::Volume(delta) => self.player.change_volume(delta),
        }
    }

    pub fn tick(&mut self) {
        self.alert_left = self.alert_left.saturating_sub(1);
        if let Some(phase) = self.timer.tick() {
            self.on_phase_started(phase);
        } else if self.music_wanted() && self.timer.seconds_left().is_multiple_of(TRACK_REFRESH_SECONDS) {
            self.track = self.player.now_playing();
        }
    }

    fn settings(&self) -> Settings {
        Settings { durations: self.timer.durations(), pack: self.pack }
    }

    /// Regra de negócio da música: só toca em foco, com o timer rodando e a música ligada.
    fn music_wanted(&self) -> bool {
        self.music_on && self.timer.is_running() && self.timer.phase() == Phase::Focus
    }

    fn sync_music(&mut self) {
        if self.music_wanted() {
            if !self.loaded {
                self.loaded = self.player.load(self.pack);
            }
            self.player.play();
        } else {
            self.player.pause();
        }
        self.track = self.player.now_playing();
    }

    fn select_pack(&mut self, pack: Pack) {
        self.pack = pack;
        self.music_on = true;
        self.store.save(&self.settings());
        self.loaded = self.player.load(pack);
        if !self.music_wanted() {
            self.player.pause();
        }
        self.track = self.player.now_playing();
    }

    fn on_phase_started(&mut self, phase: Phase) {
        self.notifier.phase_started(phase, self.timer.durations().minutes(phase));
        self.alert_left = ALERT_SECONDS;
        self.sync_music();
    }
}

/// Fakes das portas, reutilizados pelos testes da aplicação e do presenter.
#[cfg(test)]
pub mod testing {
    use super::*;
    use crate::domain::Durations;
    use std::cell::RefCell;
    use std::rc::Rc;

    pub type Log = Rc<RefCell<Vec<String>>>;

    struct FakePlayer(Log);
    impl MusicPlayer for FakePlayer {
        fn load(&mut self, p: Pack) -> bool {
            self.0.borrow_mut().push(format!("load {}", p.id()));
            true
        }
        fn play(&mut self) {
            self.0.borrow_mut().push("play".into());
        }
        fn pause(&mut self) {
            self.0.borrow_mut().push("pause".into());
        }
        fn next(&mut self) {}
        fn previous(&mut self) {}
        fn change_volume(&mut self, _: i64) {}
        fn now_playing(&mut self) -> Option<String> {
            Some("C418 – Sweden".into())
        }
    }
    struct FakeNotifier(Log);
    impl Notifier for FakeNotifier {
        fn phase_started(&self, p: Phase, m: u32) {
            self.0.borrow_mut().push(format!("notify {p:?} {m}"));
        }
    }
    struct FakeStore(Log);
    impl SettingsStore for FakeStore {
        fn save(&self, s: &Settings) {
            self.0.borrow_mut().push(format!("save {}", s.durations.minutes(Phase::Focus)));
        }
    }

    pub fn app(focus: i64) -> (Pomodoro, Log) {
        let log: Log = Default::default();
        let settings = Settings { durations: Durations::new(focus, 1), pack: Pack::Minecraft };
        let p = Pomodoro::new(settings, Box::new(FakePlayer(log.clone())), Box::new(FakeNotifier(log.clone())), Box::new(FakeStore(log.clone())));
        log.borrow_mut().clear();
        (p, log)
    }
}

#[cfg(test)]
mod tests {
    use super::testing::app;
    use super::*;

    #[test]
    fn music_follows_phases() {
        let (mut p, log) = app(1);
        p.execute(Command::Toggle);
        assert_eq!(*log.borrow(), ["load minecraft", "play"]);
        log.borrow_mut().clear();
        for _ in 0..60 {
            p.tick();
        }
        assert_eq!(*log.borrow(), ["notify Break 1", "pause"], "descanso pausa a música e avisa");
        assert!(p.is_alerting());
        log.borrow_mut().clear();
        p.execute(Command::Skip);
        assert_eq!(*log.borrow(), ["notify Focus 1", "play"], "foco retoma sem recarregar");
    }

    #[test]
    fn pause_and_music_toggle_silence_player() {
        let (mut p, log) = app(25);
        p.execute(Command::Toggle);
        p.execute(Command::Toggle);
        assert_eq!(log.borrow().last().unwrap(), "pause");
        p.execute(Command::Toggle);
        p.execute(Command::MusicToggle);
        assert_eq!(log.borrow().last().unwrap(), "pause");
        assert!(matches!(p.music(), MusicStatus::Off));
    }

    #[test]
    fn adjust_and_pack_persist() {
        let (mut p, log) = app(25);
        p.execute(Command::Adjust(Phase::Focus, 5));
        p.execute(Command::MusicPack(Pack::Witcher3));
        assert_eq!(*log.borrow(), ["save 30", "save 30", "load witcher3", "pause"], "parado: troca de pack não toca");
        assert_eq!(p.pack(), Pack::Witcher3);
    }

    #[test]
    fn parse_commands() {
        assert_eq!(Command::parse("focus +5"), Ok(Command::Adjust(Phase::Focus, 5)));
        assert_eq!(Command::parse("break -1"), Ok(Command::Adjust(Phase::Break, -1)));
        assert_eq!(Command::parse("music pack tlou"), Ok(Command::MusicPack(Pack::TheLastOfUs)));
        assert_eq!(Command::parse("vol -10"), Ok(Command::Volume(-10)));
        for bad in ["", "focus", "focus abc", "toggle now", "music pack zelda", "rm -rf"] {
            assert!(Command::parse(bad).is_err(), "{bad}");
        }
    }
}
