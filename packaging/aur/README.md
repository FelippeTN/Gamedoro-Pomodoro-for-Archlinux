# Publicar o Gamedoro no AUR

Passos para o mantenedor. O pacote se chama `gamedoro` e compila a partir do tarball da tag `vX.Y.Z` no GitHub.

## Primeira publicação

### 1. Conta e chave SSH (uma vez)

```sh
ssh-keygen -t ed25519 -f ~/.ssh/aur -C "aur"
cat >> ~/.ssh/config <<'CFG'
Host aur.archlinux.org
  IdentityFile ~/.ssh/aur
  User aur
CFG
```

Crie a conta em <https://aur.archlinux.org/register> e cole o conteúdo de `~/.ssh/aur.pub` em **My Account → SSH Public Key**.

### 2. Tag da versão no GitHub

A versão do `Cargo.toml` precisa bater com o `pkgver` do PKGBUILD.

```sh
git push origin main
git tag v0.1.0
git push origin v0.1.0
```

### 3. Checksum e teste local

```sh
cd packaging/aur
updpkgsums                    # troca o 'SKIP' pelo sha256 real do tarball da tag (pacman-contrib)
makepkg -si                   # compila, roda os testes e instala
namcap PKGBUILD *.pkg.tar.zst # opcional: sudo pacman -S namcap
```

### 4. Enviar ao AUR

```sh
git clone ssh://aur@aur.archlinux.org/gamedoro.git ~/aur/gamedoro   # o repositório começa vazio
cp PKGBUILD gamedoro.install ~/aur/gamedoro/
cd ~/aur/gamedoro
makepkg --printsrcinfo > .SRCINFO
git add PKGBUILD gamedoro.install .SRCINFO
git commit -m "gamedoro 0.1.0-1"
git push origin master
```

Depois disso, qualquer pessoa instala com `yay -S gamedoro` (ou `paru -S gamedoro`).

## Nova versão

1. Suba a `version` no `Cargo.toml`, rode `cargo build` (para atualizar o `Cargo.lock`), faça commit, crie a tag `vX.Y.Z` e dê push.
2. No PKGBUILD: `pkgver=X.Y.Z` e `pkgrel=1`.
3. `updpkgsums && makepkg -si`
4. Em `~/aur/gamedoro`: copie o PKGBUILD, rode `makepkg --printsrcinfo > .SRCINFO`, faça commit e `git push`.

Mudou só o empacotamento, sem código novo? Suba apenas o `pkgrel` (2, 3, …).

## Regras que o pacote já segue

- **Nunca escreve no `$HOME`.** Os exemplos vão para `/usr/share/gamedoro/`, e o `gamedoro.install` mostra os comandos de ativação.
- **Não distribui música.** As trilhas são baixadas pelo próprio usuário, em tempo de execução.
- **Segue as diretrizes de Rust do Arch:** `cargo fetch --locked` e `cargo build --frozen` (build reprodutível offline), e `check()` roda os testes.
- **Licença:** MIT instalada em `/usr/share/licenses/gamedoro/`.
