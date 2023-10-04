pub fn simple_hash(s: &str) -> u64 {
    let mut ret: u64 = 0;
    for c in s.chars() {
        ret += c as u64;
        ret <<= 4;
        ret |= c as u64;
    }
    ret
}

pub type HashFn = Box<dyn Fn(&str) -> u64>;

pub fn simple_hash_nonce(n: &str) -> HashFn {
    let nonce = simple_hash(n);
    Box::new(move |s| simple_hash(s) ^ nonce)
}

pub struct BenchmarkTimer {
    time: std::time::SystemTime
}

impl BenchmarkTimer {
    fn new() -> BenchmarkTimer {
        BenchmarkTimer { time: std::time::SystemTime::now() }
    }

    pub fn elapsed(&self) -> Result<String, std::time::SystemTimeError> {
        let t = self.time.elapsed()?;
        Ok(format!("{:?}", t))
    }
}

pub fn benchmark() -> Option<BenchmarkTimer> {
    if cfg!(debug_assertions) {
        Some(BenchmarkTimer::new())
    } else {
        None
    }
}


pub fn is_cli_app(name: &String) -> bool {
    matches!(
        name.as_str(),
        "ls" | "cd"
            | "cat"
            | "rm"
            | "mv"
            | "cp"
            | "mkdir"
            | "rmdir"
            | "touch"
            | "ed"
            | "if"
            | "then"
            | "else"
            | "fi"
            | "for"
            | "do"
            | "done"
            | "while"
            | "until"
            | "case"
            | "esac"
            | "vim"
            | "nano"
            | "ghc"
            | "ghci"
            | "ghcup"
            | "cabal"
            | "rustc"
            | "cargo"
            | "clang"
            | "clang++"
            | "gcc"
            | "g++"
            | "make"
            | "node"
            | "npm"
            | "yarn"
            | "pnpm"
            | "npx"
            | "python"
            | "python3"
            | "pip"
            | "pip3"
            | "ruby"
            | "gem"
            | "java"
            | "javac"
            | "jshell"
            | "javadoc"
            | "jlink"
            | "jpackage"
            | "jdeps"
            | "jmod"
            | "jdb"
            | "jconsole"
            | "git"
            | "gitk"
            | "pacman"
            | "yay"
            | "paru"
            | "apt"
            | "apt-get"
            | "tar"
            | "unzip"
            | "zip"
            | "unrar"
            | "rar"
            | "7z"
            | "zstd"
            | "gzip"
            | "gunzip"
            | "atool"
            | "neofetch"
            | "julia"
            | "nvim"
            | "emacs"
            | "htop"
            | "top"
            | "btop"
            | "nmtui"
            | "nmcli"
            | "ip"
            | "ipconfig"
            | "ifconfig"
            | "gdb"
            | "ld"
            | "alias"
            | "kill"
            | "pkill"
            | "find"
            | "tree"
            | "sudo"
            | "su"
            | "chown"
            | "chmod"
            | "grep"
            | "sed"
    )
}
