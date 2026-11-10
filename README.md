# Gamedoro

Pomodoro ultraleve para a **Waybar** no Arch Linux, com as trilhas sonoras oficiais de **Minecraft**, **The Witcher 3** e **The Last of Us**. Cada álbum é baixado do YouTube uma única vez e depois toca offline.

- Timer na barra: `󰔛 24:13` em foco, ` 04:59` no descanso e `` quando está pausado.
- O tooltip mostra o restante sobre o total da fase, uma barra de progresso, o horário de término, os tempos de foco e de descanso, o ciclo completo, os pomodoros concluídos, o tempo focado e a faixa atual.
- A música toca só durante o foco. No descanso ela pausa, e o Gamedoro avisa com som, notificação e o módulo piscando na barra.
- Parado ou pausado, ele usa **0% de CPU e 0 wakeups**, e o dispositivo de áudio fica fechado.

---

## Instalação pelo AUR

```sh
yay -S gamedoro          # ou: paru -S gamedoro
```

O pacote instala o binário em `/usr/bin` e os arquivos da Waybar em `/usr/share/gamedoro/`. Depois, faça o **passo 3** abaixo copiando de `/usr/share/gamedoro/` em vez de `waybar/`. Quem mantém o pacote encontra o guia de publicação em [packaging/aur/README.md](packaging/aur/README.md).

## Instalação passo a passo (a partir do código)

### 1. Dependências

```sh
sudo pacman -S --needed rustup pkgconf alsa-lib yt-dlp ffmpeg libnotify pipewire pipewire-alsa rofi zenity xdg-utils ttf-jetbrains-mono-nerd
rustup default stable        # só se você nunca configurou o Rust
```

| Pacote | Para quê |
|---|---|
| `rustup`, `pkgconf`, `alsa-lib` | compilar (o player de áudio usa a ALSA, que o PipeWire atende via `pipewire-alsa`) |
| `yt-dlp`, `ffmpeg` | baixar as trilhas uma vez e converter para Ogg Vorbis |
| `libnotify` | notificação de troca de fase (`notify-send`) |
| `pipewire` | `pw-play`, para o som de aviso |
| `zenity` | slider de volume para arrastar com o mouse |
| `rofi`, `xdg-utils` | menu de controle do clique direito e "abrir pasta das músicas" (o HyDE já traz o rofi) |
| `ttf-jetbrains-mono-nerd` | ícones e alinhamento do tooltip (o HyDE já traz) |

### 2. Compilar e instalar o binário

```sh
git clone <este repositório> ~/Documents/Gamedoro-Pomodoro-for-Archlinux
cd ~/Documents/Gamedoro-Pomodoro-for-Archlinux
cargo build --release
install -Dm755 target/release/gamedoro ~/.local/bin/gamedoro
```

### 3. Ligar na Waybar

**No HyDE**, o `config.jsonc` já inclui `~/.config/waybar/modules/*json*`:

```sh
mkdir -p ~/.config/waybar/modules ~/.config/waybar/menus
cp waybar/custom-gamedoro.jsonc ~/.config/waybar/modules/
cp waybar/gamedoro.rasi ~/.config/waybar/menus/
grep -q custom-gamedoro ~/.config/waybar/user-style.css || cat waybar/gamedoro.css >> ~/.config/waybar/user-style.css
```

Depois, adicione `"custom/gamedoro"` na lista `modules` de algum grupo. Faça isso **nos dois arquivos**: em `~/.config/waybar/config.jsonc` e no layout ativo em `~/.config/waybar/layouts/hyprdots/*.jsonc`, porque o HyDE regenera um a partir do outro. Exemplo:

```jsonc
"group/pill#left2": {
    "orientation": "inherit",
    "modules": [
        "custom/gamedoro",
        "idle_inhibitor",
        "clock"
    ]
}
```

**Na Waybar sem HyDE**, cole o bloco `"custom/gamedoro": { … }` de `waybar/custom-gamedoro.jsonc` dentro do seu `config.jsonc`. Ponha `"custom/gamedoro"` em `modules-left`, `modules-center` ou `modules-right`, e acrescente `waybar/gamedoro.css` ao seu `style.css`.

### 4. Recarregar

```sh
killall -SIGUSR2 waybar
```

O timer aparece na barra como `⏸ 25:00`. A waybar inicia o Gamedoro sozinha (`exec`) em todo login e o reabre se ele cair. Não precisa de serviço do systemd nem de autostart.

### 5. Primeiro uso

1. Clique no timer para começar o foco.
2. Na primeira vez, o tooltip mostra `baixando trilhas do YouTube…`. A primeira faixa começa a tocar assim que fica pronta, e as demais continuam baixando em segundo plano.
3. Para trocar de trilha ou ajustar os tempos, use o menu do clique direito.

---

## Controles

| Ação na barra | Efeito |
|---|---|
| Clique | play / pause do timer |
| Clique do meio | pular fase |
| Scroll ↑ / ↓ | foco ±1 min |
| Clique direito | abre ou fecha o menu de controle (abaixo) |

### Menu de controle

O clique direito abre um menu do rofi que **continua aberto** a cada clique: dá para apertar "+5 min" três vezes seguidas e ver o valor subir na hora. O menu é dividido em seções, e cada título mostra o valor atual:

| Seção | Itens |
|---|---|
| **FOCO** · `25 min` | +5, −5, +1, −1 min |
| **DESCANSO** · `5 min` | +1, −1 min |
| **TRILHA** · `Minecraft` | Minecraft, The Witcher 3, The Last of Us (a ativa fica marcada e destacada) |
| **MÚSICA** | liga/desliga, próxima, anterior, abrir a pasta das músicas |
| **VOLUME** | barra de volume (salva no config): **clique para abrir o slider e arrastar com o mouse**, que aplica o volume enquanto você arrasta (Cancelar desfaz). No menu, ← → ajustam 5% |
| **TIMER** | pular fase, reiniciar, fechar |

Para fechar, clique em **Fechar**, aperte `Esc`, clique fora do menu ou clique de novo com o botão direito no timer. O menu só existe enquanto está aberto, então o custo dele é zero no resto do tempo. O visual fica em `~/.config/waybar/menus/gamedoro.rasi`.

Pela CLI (dá para usar em keybinds do Hyprland):

```sh
gamedoro toggle | skip | reset
gamedoro focus +5          # ou -5; ajusta também o tempo restante se a fase for a atual
gamedoro break -1
gamedoro music toggle | next | prev | cycle
gamedoro music pack minecraft | witcher3 | tlou
gamedoro vol +10           # relativo; `gamedoro vol 70` define o valor absoluto
gamedoro volume-slider     # slider de volume para arrastar com o mouse
```

Exemplo em `~/.config/hypr/keybindings.conf`: `bind = SUPER, P, exec, gamedoro toggle`

## Música

| Pack | Fonte padrão | Faixas | Disco |
|---|---|---|---|
| Minecraft | *Minecraft – Volume Alpha*, canal oficial do C418 | 24 (59 min) | ~54 MB |
| The Witcher 3 | *The Witcher 3: Wild Hunt*, álbum oficial (Marcin Przybyłowicz – Topic) | 55 (116 min) | ~107 MB |
| The Last of Us | *The Last of Us*, álbum oficial de Gustavo Santaolalla | 30 (55 min) | ~51 MB |

- O áudio vai para `~/.cache/gamedoro/<pack>/`, em Ogg Vorbis de ~130 kbps (cerca de 0,92 MB por minuto, medido).
- Depois do download, tudo toca offline e a internet não é mais usada.
- Você também pode colocar seus próprios `.ogg`, `.mp3` ou `.flac` nessas pastas.
- Se o download for interrompido (waybar recarregada, reboot), ele **retoma de onde parou** na próxima vez que o pack for escolhido, sem baixar de novo o que já terminou. As faixas que já existem tocam enquanto isso. O arquivo `.done` na pasta marca o pack como completo.
- Para baixar do zero (por exemplo, depois de trocar a fonte), apague a pasta: `rm -rf ~/.cache/gamedoro/minecraft`.
- Para ver ou apagar as músicas baixadas, use **Abrir pasta das músicas** no menu do clique direito.

Baixar vídeos vai contra os termos de uso do YouTube. Use só para fins pessoais.

## Configuração

O arquivo `~/.config/gamedoro/config` é criado na primeira execução e é regravado quando você muda algo pela barra:

```ini
focus=25
break=5
pack=minecraft
source.minecraft=https://www.youtube.com/playlist?list=PLHykAyQQdTart3T8wrDjEnAFEmbVstInA
source.witcher3=https://www.youtube.com/playlist?list=OLAK5uy_mCEI_dk83gKOMlNZa8xX1YTT2imtILM2E
source.tlou=https://www.youtube.com/playlist?list=OLAK5uy_kyMghDo-VcuepOIbTbBETu6JT1iXvketY
```

- Cada `source.*` aceita qualquer coisa que o `yt-dlp` aceita: uma URL de playlist ou de vídeo, ou `ytsearchN:busca`.
- Para ver uma fonte antes de baixar: `yt-dlp --flat-playlist --print "%(duration_string)s | %(title)s" "<fonte>"`
- Fontes trocadas por você passam por um filtro que só aceita faixas de 30 s a 15 min, o que descarta shorts e loops de 10 horas.
- Depois de editar à mão, rode `killall -SIGUSR2 waybar`. Isso reinicia o timer.

## Atualizar

```sh
cd ~/Documents/Gamedoro-Pomodoro-for-Archlinux
git pull
cargo build --release
install -Dm755 target/release/gamedoro ~/.local/bin/gamedoro
cp waybar/custom-gamedoro.jsonc ~/.config/waybar/modules/
cp waybar/gamedoro.rasi ~/.config/waybar/menus/
rm -f ~/.config/waybar/menus/gamedoro.xml    # menu GTK das versões antigas
killall -SIGUSR2 waybar    # reinicia o timer
```

## Desinstalar

```sh
rm ~/.local/bin/gamedoro ~/.config/waybar/modules/custom-gamedoro.jsonc ~/.config/waybar/menus/gamedoro.rasi
rm -rf ~/.config/gamedoro ~/.cache/gamedoro
# remova "custom/gamedoro" do config.jsonc e do layout, e o bloco #custom-gamedoro do user-style.css
killall -SIGUSR2 waybar
```

## Problemas comuns

| Sintoma | Causa e solução |
|---|---|
| O timer não aparece na barra | Confira se `"custom/gamedoro"` está num grupo do `config.jsonc` **e** do layout do HyDE. Rode `pgrep -a gamedoro` e depois `killall -SIGUSR2 waybar` |
| Tooltip antigo depois de atualizar | A GTK guarda o tooltip em cache: rode `killall -SIGUSR2 waybar` |
| `instale o yt-dlp` | `sudo pacman -S yt-dlp`, depois escolha o pack de novo no menu |
| `download falhou` | Teste a fonte com `yt-dlp --flat-playlist "<fonte>"`. Atualize o yt-dlp (`sudo pacman -Syu yt-dlp`), porque o YouTube muda com frequência |
| Poucas faixas num pack | Escolha o pack de novo no menu para retomar o download. Se a fonte for uma busca (`ytsearch`), troque por uma playlist |
| O clique direito não abre o menu | Instale o `rofi` e confira se `~/.config/waybar/menus/gamedoro.rasi` existe. Para testar no terminal: `rofi -show gamedoro -modi "gamedoro:gamedoro rofi"` |
| `gamedoro: command not found` na waybar | O módulo chama `gamedoro` pelo PATH. Quem instalou a partir do código precisa ter `~/.local/bin` no PATH da sessão; se não tiver, troque `gamedoro` por `~/.local/bin/gamedoro` no módulo |
| Sem som, mas a faixa aparece | Confira se o `pipewire-alsa` está instalado. O erro aparece nos logs da waybar: `journalctl --user -b \| grep gamedoro` |
| `gamedoro: daemon não está rodando` | A waybar ainda não iniciou o módulo: rode `killall -SIGUSR2 waybar` |

## Custo operacional

Medido nesta máquina (Arch, kernel 7.2, PipeWire, 2 monitores), pelo `/proc/PID/{stat,status}`.

| Estado | RAM (RSS) | CPU | Observação |
|---|---|---|---|
| Parado | **3,7 MB** | **0** (0 wakeups) | bloqueia em `recv()` |
| Timer rodando, sem música | 3,7 MB | **< 0,02%** de um core | 1 wakeup/s |
| Tocando música | **~10 MB** | **~1%** de um core | decodifica Vorbis + 1 stream no PipeWire |
| Pausado depois de tocar | 6,7 MB | **0** | o dispositivo de áudio é fechado; a posição da faixa fica guardada |
| Seguidor (2º monitor) | 3,5 MB | 0 | só repassa o estado do líder |

| Outros custos | |
|---|---|
| Binário | 1,7 MB (inclui o player embutido `rodio`) |
| Comando do cliente (`gamedoro toggle`) | ~1 ms |
| Chime e notificação | `pw-play` + `notify-send`, uma vez por troca de fase |
| Download | `yt-dlp` + `ffmpeg`, **uma vez por pack**; ~212 MB em disco com os 3 packs |
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
│   ├── rofi.rs        menu de controle (script mode do rofi; continua aberto entre cliques)
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
- O spotifyd foi descartado: em 2026 o Spotify recusa as chaves de áudio para clientes baseados em librespot (`error audio key`).
