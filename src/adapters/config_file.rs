//! SettingsStore em `~/.config/gamedoro/config` (formato key=value, editável à mão).

use crate::adapters::local::DEFAULT_SOURCES;
use crate::domain::{Durations, Pack, Phase, Settings};
use crate::ports::SettingsStore;
use std::path::PathBuf;
use std::{env, fs};

#[derive(Debug, PartialEq)]
pub struct ConfigFile {
    pub settings: Settings,
    /// Fonte do yt-dlp de cada pack (URL de playlist/vídeo ou `ytsearchN:busca`).
    pub sources: [String; 3],
}

impl ConfigFile {
    pub fn path() -> PathBuf {
        env::var_os("XDG_CONFIG_HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|| PathBuf::from(env::var_os("HOME").unwrap_or_default()).join(".config"))
            .join("gamedoro/config")
    }

    pub fn load() -> ConfigFile {
        ConfigFile::parse(&fs::read_to_string(Self::path()).unwrap_or_default())
    }

    /// Linhas inválidas são ignoradas e o valor padrão é mantido.
    pub fn parse(text: &str) -> ConfigFile {
        let mut s = Settings::default();
        let mut sources = DEFAULT_SOURCES.map(String::from);
        for line in text.lines() {
            let Some((k, v)) = line.split_once('=') else { continue };
            let (k, v) = (k.trim(), v.trim());
            match k {
                "focus" => s.durations = v.parse().map(|m| s.durations.with(Phase::Focus, m)).unwrap_or(s.durations),
                "break" => s.durations = v.parse().map(|m| s.durations.with(Phase::Break, m)).unwrap_or(s.durations),
                "pack" => s.pack = Pack::from_id(v).unwrap_or(s.pack),
                _ => {
                    if let Some(p) = k.strip_prefix("source.").and_then(Pack::from_id) {
                        sources[p.index()] = v.to_string();
                    }
                }
            }
        }
        ConfigFile { settings: s, sources }
    }

    pub fn serialize(&self) -> String {
        let d: Durations = self.settings.durations;
        let mut out = format!(
            "focus={}\nbreak={}\npack={}\n",
            d.minutes(Phase::Focus),
            d.minutes(Phase::Break),
            self.settings.pack.id()
        );
        for p in Pack::ALL {
            out += &format!("source.{}={}\n", p.id(), self.sources[p.index()]);
        }
        out
    }
}

/// Persiste as preferências preservando as fontes que o usuário configurou.
pub struct FileStore {
    sources: [String; 3],
}

impl FileStore {
    pub fn new(sources: [String; 3]) -> FileStore {
        FileStore { sources }
    }
}

impl SettingsStore for FileStore {
    fn save(&self, settings: &Settings) {
        let path = ConfigFile::path();
        let text = ConfigFile { settings: *settings, sources: self.sources.clone() }.serialize();
        let res = path.parent().map_or(Ok(()), fs::create_dir_all).and_then(|_| fs::write(&path, text));
        if let Err(e) = res {
            eprintln!("gamedoro: não salvei {}: {e}", path.display());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn roundtrip_and_bad_input() {
        let mut c = ConfigFile::parse("");
        c.settings = Settings { durations: Durations::new(50, 10), pack: Pack::TheLastOfUs };
        c.sources[1] = "https://www.youtube.com/playlist?list=abc".into();
        assert_eq!(ConfigFile::parse(&c.serialize()), c);

        let bad = ConfigFile::parse("focus=lixo\nbreak=9999\npack=zelda\nsource.zelda=x\nsem_igual");
        assert_eq!(bad.settings, Settings { durations: Durations::new(25, Durations::MAX as i64), pack: Pack::Minecraft });
        assert_eq!(bad.sources, DEFAULT_SOURCES.map(String::from));
    }
}
