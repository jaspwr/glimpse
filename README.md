# glimpse
A simple GTK3 launcher/finder utility for GNU/Linux. Current features include searching for applications, directories, files by name and contents using TF-IDF, chromium and firefox bookmarks, dictionary definitions, steam games and evaluates mathematical expressions.

> This project is still in development. Some features may not work as expected. If you have any issues or suggestions, please open an issue.

https://github.com/jaspwr/glimpse/assets/40232406/7a9e1af0-b41a-4c26-a287-81bcb8bd12f8

## Installation
There are currently no packages available so you will have to install manually. This may change in the future.
### Installing a pre-built binary
1. Ensure you have the [dependencies](#dependencies) installed.
2. Download the latest release from the [releases page](https://github.com/jaspwr/glimpse/releases).
3. Extract the archive and copy the `glimpse` and `glimpse-indexer` binaries to `/usr/local/bin`.
```bash
$ tar -xvf glimpse-X.Y.Z.tar.gz
$ sudo cp glimpse-X.Y.Z/glimpse /usr/local/bin
$ sudo cp glimpse-X.Y.Z/glimpse-indexer /usr/local/bin
```
### Build from source
First ensure you have the [dependencies](#dependencies) installed. Then run the following commands:
```bash
$ git clone https://github.com/jaspwr/glimpse
$ cd glimpse
$ bash install.sh
```

## Configuration
After running for the first time, a configuration file should be created at `~/.config/glimpse/config.toml`. This file can be edited to change the default settings.

## Dependencies
* GTK3
* sqlite3
* xdg-utils
* bash
* coreutils

### Make
* Rust & Cargo
* Git
