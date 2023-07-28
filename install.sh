#!/bin/bash
cargo build --release &&
sudo cp target/release/glimpse /usr/local/bin &&
sudo cp target/release/glimpse-indexer /usr/local/bin &&
echo "Installed."
