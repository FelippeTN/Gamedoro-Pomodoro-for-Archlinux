# Gamedoro

Pomodoro ultraleve para a **Waybar** no Arch Linux, com trilhas sonoras de **Minecraft**, **The Witcher 3** e **The Last of Us**. As trilhas são baixadas do YouTube uma única vez e tocadas offline.

- Timer na barra: `󰔛 24:13` em foco, ` 04:59` no descanso e `` quando está pausado.
- O tooltip mostra o restante sobre o total da fase, uma barra de progresso, o horário de término, os tempos de foco e de descanso, o ciclo completo, os pomodoros concluídos, o tempo focado e a faixa atual.
- A música toca só durante o foco. No descanso ela pausa, e o Gamedoro avisa com som, notificação e o módulo piscando na barra.
- Parado ou pausado, ele usa **0% de CPU e 0 wakeups**, e o dispositivo de áudio fica fechado.

## Controles

| Ação na barra | Efeito |
|---|---|
| Clique | play / pause do timer |
| Clique do meio | pular fase |
| Scroll ↑ / ↓ | foco ±1 min |
| Clique direito | menu: trilha sonora, música (on/off, próxima, anterior, volume), tempo de foco ±5, descanso ±1, pular, reiniciar |

Pela CLI (dá para usar em keybinds do Hyprland):

```sh
gamedoro toggle | skip | reset
gamedoro focus +5          # ou -5; ajusta também o tempo restante se a fase for a atual
gamedoro break -1
gamedoro music toggle | next | prev | cycle
gamedoro music pack minecraft | witcher3 | tlou
gamedoro vol +10
```

Exemplo de keybind: `bind = SUPER, P, exec, gamedoro toggle`

## Instalação

```sh
sudo pacman -S yt-dlp ffmpeg      # yt-dlp baixa; ffmpeg converte para Ogg Vorbis

cargo build --release
install -Dm755 target/release/gamedoro ~/.local/bin/gamedoro

# Waybar (HyDE): o glob modules/*json* já carrega o módulo
cp waybar/custom-gamedoro.jsonc ~/.config/waybar/modules/
cp waybar/gamedoro.xml ~/.config/waybar/menus/
cat waybar/gamedoro.css >> ~/.config/waybar/user-style.css
# adicione "custom/gamedoro" em algum grupo do config.jsonc (e no layout do HyDE)
killall -SIGUSR2 waybar
```

A waybar inicia o Gamedoro sozinha (`exec`) e o reabre se ele cair. Não precisa de serviço nem de autostart.

## Música

Quando você escolhe um pack pela primeira vez, ou dá play no foco, o Gamedoro roda o `yt-dlp` em segundo plano:

- O áudio vai para `~/.cache/gamedoro/<pack>/`, em Ogg Vorbis de cerca de 130 kbps.
- A primeira faixa começa assim que fica pronta, e o tooltip mostra `baixando…` enquanto o download continua.
- Só entram faixas de 1 a 15 min, o que ignora shorts e compilações de 10 horas.
- Depois do download, tudo toca offline e a internet não é mais usada.

Você também pode colocar seus próprios `.ogg`, `.mp3` ou `.flac` nessas pastas. Ao trocar uma fonte, apague a pasta do pack para baixar de novo.

Baixar vídeos vai contra os termos de uso do YouTube. Use só para fins pessoais.

## Configuração

O arquivo `~/.config/gamedoro/config` é criado na primeira execução e é regravado quando você muda algo pela barra:

```ini
focus=25
break=5
pack=minecraft
source.minecraft=ytsearch15:C418 Minecraft Volume Alpha
source.witcher3=ytsearch15:The Witcher 3 Wild Hunt original soundtrack
source.tlou=ytsearch15:Gustavo Santaolalla The Last of Us soundtrack
```

Cada `source.*` aceita qualquer coisa que o `yt-dlp` aceita: uma URL de playlist ou de vídeo, ou `ytsearchN:busca` (os N primeiros resultados). Depois de editar à mão, rode `killall -SIGUSR2 waybar`.

## Custo operacional

Medido nesta máquina (Arch, kernel 7.2, PipeWire, 2 monitores), pelo `/proc/PID/{stat,status}`.

| Estado | RAM (RSS) | CPU | Observação |
|---|---|---|---|
| Parado | **3,7 MB** | **0** (0 wakeups) | bloqueia em `recv()` |
| Timer rodando, sem música | 3,7 MB | **< 0,02%** de um core | 1 wakeup/s |
| Tocando música | **9,7 MB** | **~1%** de um core | decodifica Vorbis + 1 stream no PipeWire |
| Pausado depois de tocar | 6,7 MB | **0** | o dispositivo de áudio é fechado; a posição da faixa fica guardada |
| Seguidor (2º monitor) | 2,3 MB | 0 | só repassa o estado do líder |

| Outros custos | |
|---|---|
| Binário | 1,7 MB (inclui o player embutido `rodio`) |
| Comando do cliente (`gamedoro toggle`) | ~1 ms |
| Chime e notificação | `pw-play` + `notify-send`, uma vez por troca de fase |
| Download | `yt-dlp` + `ffmpeg`, **uma vez por pack** (~60 MB por pack em disco) |
| Rede | só durante esse download |

## Arquitetura

O código segue DDD em camadas com inversão de dependência. O domínio não conhece YouTube, áudio, Waybar nem arquivos.

```
src/
├── domain.rs        Timer (aggregate), Phase, Pack, Durations, Settings: regras puras, sem IO
├── ports.rs         traits MusicPlayer, Notifier, SettingsStore
├── app.rs           Pomodoro: casos de uso (Command), regra "música só no foco"
├── adapters/
│   ├── local.rs       MusicPlayer → yt-dlp (download único) + rodio (playback offline)
│   ├── desktop.rs     Notifier → pw-play + notify-send
│   ├── config_file.rs SettingsStore → ~/.config/gamedoro/config
│   ├── clock.rs       hora local (para o "termina às")
│   └── waybar.rs      presenter: estado → JSON + tooltip Pango centralizado
├── daemon.rs        socket Unix, eleição de líder (flock), loop de 1 s
└── main.rs          composition root + CLI
```

- **Uma waybar por monitor** roda um `exec` em cada monitor. A instância que pega o `flock` vira líder e as outras só repassam o estado dela. Se o líder morrer, um seguidor assume.
- **Trocar a fonte de música** (MPD, Spotify oficial via MPRIS) é implementar `MusicPlayer` num adapter novo e trocar uma linha no `main.rs`.
- **Segurança:**
  - O socket de controle fica em `$XDG_RUNTIME_DIR` com modo `0600`, e todo comando que chega por ele é validado de novo.
  - A fonte vai para o `yt-dlp` como argumento separado e depois de `--`, sem passar por um shell.

```sh
cargo test     # domínio, casos de uso (com player fake), parser, config, cache, tooltip
```

## Limitações conhecidas

- Depois de um suspend, o timer retoma de onde parou e não desconta o tempo dormido.
- Não há descanso longo a cada 4 ciclos, e a contagem de pomodoros zera quando a waybar reinicia.
- A busca `ytsearch` pode trazer alguma faixa que não é da trilha oficial. Para ter controle total, use a URL de uma playlist.
- O spotifyd foi descartado: em 2026 o Spotify recusa as chaves de áudio para clientes baseados em librespot (`error audio key`).
