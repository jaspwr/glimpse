#!/bin/bash
cargo build --release --features app &&
sudo cp target/release/glimpse /usr/local/bin &&
sudo cp target/release/glimpse-indexer /usr/local/bin &&
echo "Installed."
