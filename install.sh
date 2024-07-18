#!/bin/bash

# echo "Installing for user $(whoami)."
# echo "Continue? [y/n]"
# read -r response
# if [ "$response" != "y" ]; then
# 	echo "Aborted."
# 	exit 1
# fi

cargo build --release --features app &&

./target/release/glimpse-indexer --init &&

installdir="/usr/local/bin"

sudo cp target/release/glimpse $installdir &&
sudo cp target/release/glimpse-indexer $installdir &&

# sudo cp target/release/glimpse-monitor $installdir &&
#
# echo "[Unit]
# Description=File change monitor for Glimpse
# After=network.target
#
# [Service]
# ExecStart=$installdir/glimpse-monitor
# Restart=always
# User=root
# Group=root
#
# [Install]
# WantedBy=multi-user.target
# " > glimpse-monitor.service

# sudo systemctl daemon-reload &&
#
# sudo systemctl enable glimpse-monitor.service
# sudo systemctl start glimpse-monitor.service

echo "Installed."
