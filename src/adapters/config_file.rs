//! SettingsStore em `~/.config/gamedoro/config` (formato key=value, editável à mão).

use crate::adapters::local::DEFAULT_SOURCES;
use crate::domain::{Durations, Pack, Phase, Settings};
use crate::ports::SettingsStore;
use std::io::{self, ErrorKind};
use std::path::{Path, PathBuf};
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

    /// Só leitura (menu do rofi): se o arquivo não abre, mostra os padrões.
    pub fn load() -> ConfigFile {
        Self::read(&Self::path()).unwrap_or_else(|_| Self::parse(""))
    }

    /// Arquivo ausente = padrões. Outros erros (permissão, UTF-8 inválido) sobem para quem chamou.
    fn read(path: &Path) -> io::Result<ConfigFile> {
        match fs::read_to_string(path) {
            Ok(text) => Ok(Self::parse(&text)),
            Err(e) if e.kind() == ErrorKind::NotFound => Ok(Self::parse("")),
            Err(e) => Err(e),
        }
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
    /// None = o arquivo existe mas não deu para ler: não salva, senão os padrões apagariam o que o usuário escreveu.
    path: Option<PathBuf>,
    sources: [String; 3],
}

impl FileStore {
    /// Lê o config e devolve o store que o persiste.
    pub fn open() -> (ConfigFile, FileStore) {
        Self::open_at(ConfigFile::path())
    }

    fn open_at(path: PathBuf) -> (ConfigFile, FileStore) {
        match ConfigFile::read(&path) {
            Ok(cfg) => {
                let store = FileStore { path: Some(path), sources: cfg.sources.clone() };
                (cfg, store)
            }
            Err(e) => {
                eprintln!("gamedoro: não li {}: {e}; usando padrões sem sobrescrever o arquivo", path.display());
                let cfg = ConfigFile::parse("");
                let store = FileStore { path: None, sources: cfg.sources.clone() };
                (cfg, store)
            }
        }
    }
}

impl SettingsStore for FileStore {
    fn save(&self, settings: &Settings) {
        let Some(path) = &self.path else { return };
        let text = ConfigFile { settings: *settings, sources: self.sources.clone() }.serialize();
        // temporário + rename: uma queda no meio da escrita nunca deixa o config cortado
        let tmp = path.with_extension("tmp");
        let res = path
            .parent()
            .map_or(Ok(()), fs::create_dir_all)
            .and_then(|_| fs::write(&tmp, text))
            .and_then(|_| fs::rename(&tmp, path));
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

    #[test]
    fn unreadable_config_is_never_overwritten() {
        let dir = env::temp_dir().join(format!("gamedoro-cfg-test-{}", std::process::id()));
        fs::create_dir_all(&dir).unwrap();
        let path = dir.join("config");
        let saved = Settings { durations: Durations::new(40, 5), pack: Pack::Minecraft };

        // controle: arquivo ausente vira padrão e o save cria o arquivo
        let (cfg, store) = FileStore::open_at(path.clone());
        assert_eq!(cfg, ConfigFile::parse(""));
        store.save(&saved);
        assert_eq!(FileStore::open_at(path.clone()).0.settings, saved);
        assert!(!path.with_extension("tmp").exists(), "temporário renomeado");

        // UTF-8 inválido: lê padrões, mas o save não toca no arquivo do usuário
        let original = b"focus=40\nsource.minecraft=minha\n\xff\n";
        fs::write(&path, original).unwrap();
        let (cfg, store) = FileStore::open_at(path.clone());
        assert_eq!(cfg.settings, Settings::default());
        store.save(&cfg.settings);
        let after = fs::read(&path).unwrap();
        fs::remove_dir_all(&dir).unwrap();
        assert_eq!(after, original);
    }
}
