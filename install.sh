#!/bin/bash

echo "Installing for user $(whoami)."
echo "Continue? [y/n]"
read -r response
if [ "$response" != "y" ]; then
	echo "Aborted."
	exit 1
fi

cargo build --release --features app &&

./target/release/glimpse-indexer --init &&

sudo cp target/release/glimpse /usr/local/bin &&
sudo cp target/release/glimpse-indexer /usr/local/bin &&
sudo cp target/release/glimpse-monitor /usr/local/bin &&

sudo cp glimpse-monitor.service /etc/systemd/system/ &&

sudo systemctl daemon-reload &&

sudo systemctl enable glimpse-monitor.service
sudo systemctl start glimpse-monitor.service

echo "Installed."
