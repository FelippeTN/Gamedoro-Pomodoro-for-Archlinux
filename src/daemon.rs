//! Infraestrutura de execução: socket Unix de controle, eleição de líder entre as instâncias
//! (uma waybar por monitor) e o loop de 1 s que só acorda quando há algo correndo.

use crate::adapters::{clock, waybar};
use crate::app::{Command, Pomodoro};
use std::fs::{self, File};
use std::io::{BufRead, BufReader, Write};
use std::os::unix::fs::PermissionsExt;
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::mpsc::{self, RecvTimeoutError};
use std::time::{Duration, Instant};
use std::{env, thread};

const SEC: Duration = Duration::from_secs(1);

pub fn socket_path() -> PathBuf {
    runtime_dir().join("gamedoro.sock")
}

fn runtime_dir() -> PathBuf {
    env::var_os("XDG_RUNTIME_DIR").map(PathBuf::from).unwrap_or_else(env::temp_dir)
}

/// Envia um comando ao líder (lado cliente).
pub fn send(line: &str) -> std::io::Result<()> {
    writeln!(UnixStream::connect(socket_path())?, "{line}")
}

enum Msg {
    Cmd(String),
    Watch(UnixStream),
}

/// Quem pega o lock vira líder e roda o Pomodoro; os demais só repassam o estado do líder.
/// Se o líder morrer, um seguidor assume.
pub fn run(build: impl Fn() -> Pomodoro) -> ! {
    let sock = socket_path();
    let lock = File::create(runtime_dir().join("gamedoro.lock")).unwrap_or_else(|e| fail(&format!("lock: {e}")));
    loop {
        if lock.try_lock().is_ok() {
            let _ = fs::remove_file(&sock); // socket velho de um líder morto
            let listener = UnixListener::bind(&sock).unwrap_or_else(|e| fail(&format!("bind {}: {e}", sock.display())));
            let _ = fs::set_permissions(&sock, fs::Permissions::from_mode(0o600));
            lead(listener, build());
        }
        if let Ok(mut s) = UnixStream::connect(&sock) {
            if writeln!(s, "watch").is_ok() {
                for line in BufReader::new(s).lines() {
                    let Ok(line) = line else { break };
                    print_or_exit(&line);
                }
            }
        }
        thread::sleep(Duration::from_millis(300)); // líder caiu ou ainda subindo
    }
}

fn lead(listener: UnixListener, mut app: Pomodoro) -> ! {
    let (tx, rx) = mpsc::channel();
    thread::spawn(move || {
        for s in listener.incoming().flatten() {
            let _ = s.set_read_timeout(Some(SEC));
            let _ = s.set_write_timeout(Some(SEC));
            let mut line = String::new();
            if BufReader::new(&s).read_line(&mut line).is_err() {
                continue;
            }
            let line = line.trim();
            let msg = if line == "watch" { Msg::Watch(s) } else { Msg::Cmd(line.to_string()) };
            if tx.send(msg).is_err() {
                break;
            }
        }
    });

    let mut watchers: Vec<UnixStream> = vec![];
    let mut next = Instant::now() + SEC;
    loop {
        print_or_exit(&broadcast(&mut watchers, waybar::render(&app, clock::seconds_of_day())));
        let ticking = app.needs_ticks();
        let msg = if ticking {
            match rx.recv_timeout(next.saturating_duration_since(Instant::now())) {
                Ok(m) => Some(m),
                Err(RecvTimeoutError::Timeout) => None,
                Err(RecvTimeoutError::Disconnected) => fail("listener morreu"),
            }
        } else {
            Some(rx.recv().unwrap_or_else(|_| fail("listener morreu"))) // parado: 0 wakeups
        };
        match msg {
            Some(Msg::Watch(s)) => watchers.push(s),
            // o socket é fronteira de confiança: revalida tudo que chega
            Some(Msg::Cmd(line)) => match Command::parse(&line) {
                Ok(cmd) => {
                    app.execute(cmd);
                    if !ticking {
                        next = Instant::now() + SEC;
                    }
                }
                Err(e) => eprintln!("gamedoro: {e}"),
            },
            None => {
                let now = Instant::now();
                // shortcut: após suspend o timer não "recupera" o tempo dormido, só retoma.
                next = if now > next + 2 * SEC { now + SEC } else { next + SEC };
                app.tick();
            }
        }
    }
}

fn broadcast(watchers: &mut Vec<UnixStream>, line: String) -> String {
    watchers.retain_mut(|w| writeln!(w, "{line}").is_ok());
    line
}

fn print_or_exit(line: &str) {
    if writeln!(std::io::stdout(), "{line}").is_err() {
        std::process::exit(0); // waybar fechou o pipe
    }
}

fn fail(msg: &str) -> ! {
    eprintln!("gamedoro: {msg}");
    std::process::exit(1)
}
