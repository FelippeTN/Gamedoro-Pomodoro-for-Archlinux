# Gamedoro

Pomodoro ultraleve para a **Waybar** no Arch Linux, com trilhas sonoras de **Minecraft**, **The Witcher 3** e **The Last of Us** tocando pelo Spotify (via `spotifyd`).

- Timer na barra: `󰔛 24:13` em foco, ` 04:59` no descanso e `` quando está pausado.
- A música toca só durante o foco. No descanso ela pausa, e o Gamedoro avisa com som, notificação e o módulo piscando na barra.
- O binário Rust tem 429 KB e **nenhuma dependência de crate**. Parado, ele usa **0% de CPU e 0 wakeups**.

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
cargo build --release
install -Dm755 target/release/gamedoro ~/.local/bin/gamedoro

# Waybar (HyDE): o glob modules/*json* já carrega o módulo
cp waybar/custom-gamedoro.jsonc ~/.config/waybar/modules/
cp waybar/gamedoro.xml ~/.config/waybar/menus/
cat waybar/gamedoro.css >> ~/.config/waybar/user-style.css
# adicione "custom/gamedoro" em algum grupo do config.jsonc (e no layout do HyDE)
killall -SIGUSR2 waybar
```

### Música (spotifyd, exige Spotify Premium)

```sh
sudo pacman -S spotifyd
spotifyd authenticate                      # OAuth no navegador, feito uma vez; a senha não é salva
mkdir -p ~/.config/spotifyd && cp waybar/spotifyd.conf ~/.config/spotifyd/
systemctl --user enable --now spotifyd
```

Sem o spotifyd, o timer funciona normalmente e o tooltip mostra `spotifyd off`.

**Segurança:** o Gamedoro nunca vê credenciais e não acessa a rede. Ele só fala com o spotifyd pelo D-Bus local (`playerctl`). O socket de controle fica em `$XDG_RUNTIME_DIR` com modo `0600`, e todo comando que chega por ele é validado de novo. O spotifyd usa o `librespot`, que é um cliente não oficial. Pelos termos de uso do Spotify isso é zona cinzenta, mas o librespot é amplamente usado.

## Configuração

O arquivo `~/.config/gamedoro/config` é criado na primeira execução e é regravado quando você muda algo pela barra:

```ini
focus=25
break=5
pack=minecraft
uri.minecraft=spotify:album:3Gt7rOjcZQoHCfnKl5AkK7
uri.witcher3=spotify:album:6pYNEn4tMc6gdv5fIZf5yn
uri.tlou=spotify:album:2GFFxj8aR2XpwIMYanOPjh
```

Cada `uri.*` aceita qualquer álbum ou playlist (no Spotify: Compartilhar → Copiar link, e troque `https://open.spotify.com/album/ID` por `spotify:album:ID`). Depois de editar à mão, reinicie com `killall -SIGUSR2 waybar`.

## Custo operacional

Medido nesta máquina (Arch, kernel 7.2, 2 monitores), pelo `/proc/PID/{stat,status}` e pela soma das trocas de contexto de todas as threads.

| Recurso | Valor medido |
|---|---|
| Binário | **429 KB**, dinâmico só contra a libc |
| RAM por instância (RSS) | **2,4 MB**, dos quais **152 KB** são memória privada. O resto é libc e o binário, compartilhados |
| Instâncias | 1 por monitor: 1 líder (2 threads) + seguidores (1 thread) |
| CPU parado (30 s) | **0 ms**, **0 wakeups** (bloqueia em `recv()`) |
| CPU rodando (60 s) | **< 10 ms** (abaixo da resolução de 1 tick), ou seja **< 0,02% de um core**; cerca de 1,2 wakeups/s |
| Comando do cliente (`gamedoro toggle`) | ~1 ms |
| `playerctl` por chamada | ~6 ms. Só roda em comando, troca de fase e a cada 15 s com música tocando |
| Chime e notificação | `pw-play` + `notify-send`, uma vez por troca de fase |
| Disco | o binário e um config de 200 bytes |
| Rede | nenhuma (o Gamedoro não acessa a rede) |

**O spotifyd é cobrado à parte e só enquanto está rodando.** Não medi aqui porque ele ainda não está instalado. Rode `grep VmRSS /proc/$(pgrep spotifyd)/status` depois de instalar. Para efeito de comparação, o app oficial do Spotify usa de 300 a 500 MB.

## Arquitetura

O código segue DDD em camadas com inversão de dependência. O domínio não conhece Spotify, Waybar nem arquivos.

```
src/
├── domain.rs        Timer (aggregate), Phase, Pack, Durations, Settings: regras puras, sem IO
├── ports.rs         traits MusicPlayer, Notifier, SettingsStore
├── app.rs           Pomodoro: casos de uso (Command), regra "música só no foco"
├── adapters/
│   ├── spotify.rs     MusicPlayer → playerctl/MPRIS → spotifyd
│   ├── desktop.rs     Notifier → pw-play + notify-send
│   ├── config_file.rs SettingsStore → ~/.config/gamedoro/config
│   └── waybar.rs      presenter: estado → JSON da Waybar
├── daemon.rs        socket Unix, eleição de líder (flock), loop de 1 s
└── main.rs          composition root + CLI
```

- **Uma waybar por monitor** roda um `exec` em cada monitor. A instância que pega o `flock` vira líder e as outras só repassam o estado dela. Se o líder morrer, um seguidor assume.
- **Trocar a fonte de música** (MPD, outro player MPRIS) é implementar `MusicPlayer` num adapter novo e trocar uma linha no `main.rs`.

```sh
cargo test     # domínio, casos de uso (com player fake), parser, config, escaping
```

## Limitações conhecidas

- Depois de um suspend, o timer retoma de onde parou e não desconta o tempo dormido.
- Não há descanso longo a cada 4 ciclos, e a contagem de pomodoros zera quando a waybar reinicia.
- Não verifiquei se as URIs padrão dos álbuns abrem a versão certa de cada região. Se alguma falhar, troque no config.
