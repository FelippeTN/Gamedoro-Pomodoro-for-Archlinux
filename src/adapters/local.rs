//! MusicPlayer local: baixa cada pack uma vez com `yt-dlp` para ~/.cache/gamedoro/<pack>/
//! e toca offline com `rodio`. O dispositivo de áudio só fica aberto enquanto toca.

use crate::domain::Pack;
use crate::ports::MusicPlayer;
use rodio::{Decoder, DeviceSinkBuilder, MixerDeviceSink, Player};
use std::fs::{self, File};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::time::Duration;
use std::env;

/// Fonte de cada pack: qualquer URL ou busca aceita pelo yt-dlp. Padrão = álbuns oficiais
/// (canal do artista / "Topic" do YouTube Music): faixas certas, na ordem certa, sempre as mesmas.
pub const DEFAULT_SOURCES: [&str; 3] = [
    // C418 – Minecraft – Volume Alpha (canal oficial do C418), 24 faixas
    "https://www.youtube.com/playlist?list=PLHykAyQQdTart3T8wrDjEnAFEmbVstInA",
    // Marcin Przybyłowicz – The Witcher 3: Wild Hunt (álbum oficial, Topic), 55 faixas
    "https://www.youtube.com/playlist?list=OLAK5uy_mCEI_dk83gKOMlNZa8xX1YTT2imtILM2E",
    // Gustavo Santaolalla – The Last of Us (álbum oficial), 30 faixas
    "https://www.youtube.com/playlist?list=OLAK5uy_kyMghDo-VcuepOIbTbBETu6JT1iXvketY",
];
const AUDIO_EXT: [&str; 3] = ["ogg", "mp3", "flac"];
/// Proteção para fontes trocadas pelo usuário: ignora shorts e compilações de horas.
const DURATION_FILTER: &str = "duration > 30 & duration < 900";
const DEFAULT_VOLUME: f32 = 0.6;
/// Marcador de download concluído; sem ele, o próximo `load` retoma o que faltou.
const DONE: &str = ".done";
/// Registro do yt-dlp do que já foi baixado (permite retomar sem baixar de novo).
const ARCHIVE: &str = ".archive";

pub struct LocalPlayer {
    root: PathBuf,
    sources: [String; 3],
    pack: Option<Pack>,
    tracks: Vec<PathBuf>,
    index: usize,
    /// Onde retomar a faixa atual (o dispositivo é fechado no pause).
    resume_at: Duration,
    volume: f32,
    wants_play: bool,
    output: Option<(MixerDeviceSink, Player)>,
    download: Option<Child>,
    error: Option<&'static str>,
}

impl LocalPlayer {
    pub fn new(sources: [String; 3]) -> LocalPlayer {
        LocalPlayer {
            root: cache_root(),
            sources,
            pack: None,
            tracks: vec![],
            index: 0,
            resume_at: Duration::ZERO,
            volume: DEFAULT_VOLUME,
            wants_play: false,
            output: None,
            download: None,
            error: None,
        }
    }

    fn dir(&self, pack: Pack) -> PathBuf {
        self.root.join(pack.id())
    }

    fn rescan(&mut self) {
        if let Some(p) = self.pack {
            self.tracks = scan(&self.dir(p));
        }
    }

    fn start_download(&mut self, pack: Pack) {
        let dir = self.dir(pack);
        if let Err(e) = fs::create_dir_all(&dir) {
            eprintln!("gamedoro: {}: {e}", dir.display());
            self.error = Some("sem permissão no cache");
            return;
        }
        let spawned = Command::new("yt-dlp")
            .args(["--quiet", "--no-warnings", "--no-progress", "--ignore-errors", "--no-overwrites"])
            .args(["-x", "--audio-format", "vorbis", "--audio-quality", "5"])
            .args(["--match-filter", DURATION_FILTER])
            .arg("--download-archive")
            .arg(dir.join(ARCHIVE))
            .arg("-o")
            .arg(dir.join("%(playlist_index)02d - %(title)s.%(ext)s"))
            .arg("--")
            .arg(&self.sources[pack.index()])
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn();
        match spawned {
            Ok(child) => self.download = Some(child),
            Err(_) => self.error = Some("instale o yt-dlp: sudo pacman -S yt-dlp"),
        }
    }

    /// Abre o dispositivo e toca a faixa atual a partir de `resume_at`.
    fn start_track(&mut self) {
        self.output = None;
        let Some(path) = self.tracks.get(self.index) else { return };
        let opened = DeviceSinkBuilder::open_default_sink().map_err(|e| e.to_string()).and_then(|mut sink| {
            sink.log_on_drop(false);
            let source = Decoder::try_from(File::open(path).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
            let player = Player::connect_new(sink.mixer());
            player.set_volume(self.volume);
            player.append(source);
            if !self.resume_at.is_zero() {
                let _ = player.try_seek(self.resume_at);
            }
            Ok((sink, player))
        });
        match opened {
            Ok(out) => {
                self.output = Some(out);
                self.error = None;
            }
            Err(e) => {
                eprintln!("gamedoro: não toquei {}: {e}", path.display());
                self.error = Some("falha ao tocar a faixa");
            }
        }
    }

    fn skip(&mut self, forward: bool) {
        if self.tracks.is_empty() {
            return;
        }
        let n = self.tracks.len();
        self.index = if forward { (self.index + 1) % n } else { (self.index + n - 1) % n };
        self.resume_at = Duration::ZERO;
        if self.wants_play {
            self.start_track();
        }
    }
}

impl MusicPlayer for LocalPlayer {
    fn load(&mut self, pack: Pack) -> bool {
        // mesmo pack completo ou baixando: nada a fazer; senão (yt-dlp instalado depois, download
        // interrompido) tenta de novo
        let complete = !self.tracks.is_empty() && self.dir(pack).join(DONE).exists();
        if self.pack == Some(pack) && (complete || self.download.is_some()) {
            return true;
        }
        self.output = None;
        self.pack = Some(pack);
        self.index = 0;
        self.resume_at = Duration::ZERO;
        self.error = None;
        if let Some(mut old) = self.download.take() {
            let _ = old.kill(); // download do pack anterior
            let _ = old.wait();
        }
        self.rescan();
        // shortcut: se o daemon morrer por sinal no meio do download, o yt-dlp segue como órfão e um
        // segundo pode começar junto; o archive evita rebaixar o que já terminou. Upgrade: PR_SET_PDEATHSIG.
        if self.tracks.is_empty() || !self.dir(pack).join(DONE).exists() {
            self.start_download(pack); // retoma; as faixas que já existem tocam enquanto isso
        }
        self.error.is_none() || !self.tracks.is_empty()
    }

    fn play(&mut self) {
        self.wants_play = true;
        if self.output.is_none() {
            self.start_track(); // sem faixas ainda: o poll começa quando o download entregar a 1ª
        }
    }

    fn pause(&mut self) {
        self.wants_play = false;
        if let Some((_, player)) = self.output.take() {
            self.resume_at = player.get_pos(); // fecha o dispositivo: pausado não gasta nada
        }
    }

    fn next(&mut self) {
        self.skip(true);
    }

    fn previous(&mut self) {
        self.skip(false);
    }

    fn change_volume(&mut self, percent: i64) {
        self.volume = (self.volume + percent as f32 / 100.0).clamp(0.0, 1.0);
        if let Some((_, player)) = &self.output {
            player.set_volume(self.volume);
        }
    }

    fn now_playing(&mut self) -> Option<String> {
        if let Some(e) = self.error {
            return Some(e.to_string());
        }
        let downloading = self.download.is_some();
        Some(match self.tracks.get(self.index) {
            Some(path) => {
                let name = track_name(path);
                let more = if downloading { " · baixando mais" } else { "" };
                format!("{name}  ({}/{}){more}", self.index + 1, self.tracks.len())
            }
            None if downloading => "baixando trilhas do YouTube…".to_string(),
            None => "escolha um pack no menu".to_string(),
        })
    }

    fn poll(&mut self) {
        if let Some(child) = &mut self.download {
            let status = child.try_wait();
            self.rescan();
            if !matches!(status, Ok(None)) {
                self.download = None;
                // terminou sozinho (não por sinal): marca como completo, mesmo com faixas indisponíveis
                let exited = matches!(status, Ok(Some(s)) if s.code().is_some());
                if let (true, Some(p)) = (exited && !self.tracks.is_empty(), self.pack) {
                    let _ = File::create(self.dir(p).join(DONE));
                }
                if self.tracks.is_empty() {
                    self.error = Some("download falhou: veja a fonte no config");
                }
            }
        }
        let track_ended = self.output.as_ref().is_some_and(|(_, p)| p.empty());
        if track_ended {
            self.skip(true);
        } else if self.wants_play && self.output.is_none() && !self.tracks.is_empty() && self.error.is_none() {
            self.start_track(); // após erro, só tenta de novo com ação do usuário (play/next/pack)
        }
    }

    fn busy(&self) -> bool {
        self.download.is_some()
    }
}

impl Drop for LocalPlayer {
    fn drop(&mut self) {
        if let Some(mut child) = self.download.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}

/// Pasta das músicas baixadas: `$XDG_CACHE_HOME/gamedoro` (padrão `~/.cache/gamedoro`).
pub fn cache_root() -> PathBuf {
    env::var_os("XDG_CACHE_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".cache"))
        .join("gamedoro")
}

/// Faixas prontas, em ordem. Ignora os temporários do yt-dlp (`.part`, `.temp.ogg`).
fn scan(dir: &Path) -> Vec<PathBuf> {
    let mut tracks: Vec<PathBuf> = fs::read_dir(dir)
        .into_iter()
        .flatten()
        .flatten()
        .map(|e| e.path())
        .filter(|p| is_track(p))
        .collect();
    tracks.sort();
    tracks
}

fn is_track(p: &Path) -> bool {
    let ext_ok = p.extension().and_then(|e| e.to_str()).is_some_and(|e| AUDIO_EXT.contains(&e));
    let temp = p.file_stem().and_then(|s| s.to_str()).is_some_and(|s| s.ends_with(".temp"));
    ext_ok && !temp
}

/// "03 - C418 - Sweden.ogg" → "C418 - Sweden"
fn track_name(p: &Path) -> String {
    let stem = p.file_stem().and_then(|s| s.to_str()).unwrap_or("?");
    match stem.split_once(" - ") {
        Some((n, rest)) if n.chars().all(|c| c.is_ascii_digit()) => rest.to_string(),
        _ => stem.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn scan_ignores_temp_and_sorts() {
        let dir = env::temp_dir().join(format!("gamedoro-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        for f in ["02 - B.ogg", "01 - A.mp3", "03 - C.temp.ogg", "04 - D.webm.part", "capa.jpg"] {
            File::create(dir.join(f)).unwrap();
        }
        let names: Vec<String> = scan(&dir).iter().map(|p| track_name(p)).collect();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(names, ["A", "B"]);
    }

    #[test]
    fn names() {
        assert_eq!(track_name(Path::new("/x/03 - C418 - Sweden.ogg")), "C418 - Sweden");
        assert_eq!(track_name(Path::new("/x/minha faixa.mp3")), "minha faixa");
    }
}
