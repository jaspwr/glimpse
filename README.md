# This project is unfinished.
# glimpse
A simple GTK3 launcher/finder utility for GNU/Linux. Current features include searching for applications, directories, files by name and contents using TF-IDF, chromium and firefox bookmarks, dictionary definitions, steam games and evaluates mathematical expressions.

> This project is still in development. Some features may not work as expected and may not be localised. If you have any issues or suggestions, please open an issue.

https://github.com/jaspwr/glimpse/assets/40232406/9bfcb16e-8e4f-4548-aa89-cc35f886ca17

## Installation
There are currently no packages available so you will have to install manually. This may change in the future.
### Installing a pre-built binary
1. Ensure you have the [dependencies](#dependencies) installed.
2. Download the latest release from the [releases page](https://github.com/jaspwr/glimpse/releases).
3. Extract the archive and copy the binaries to `/usr/local/bin`.
```bash
$ tar -xvf glimpse-X.Y.Z.tar.gz
$ sudo cp -a glimpse-X.Y.Z/ /usr/local/bin
```
### Build from source
First ensure you have the [dependencies](#dependencies) installed. Then run the following commands:
```bash
$ git clone https://github.com/jaspwr/glimpse
$ cd glimpse
$ bash install.sh
```

## Configuration
After running for the first time, a configuration file should be created at `~/.config/glimpse/config.toml` and a stylesheet at `~/.config/glimpse/styles.css`.

## Dependencies
* GTK3
* xdg-utils
* bash
* coreutils
* sqlite3 (optional; adds support for firefox bookmark search)

### Make
* Rust & Cargo
* Git
* Bash
