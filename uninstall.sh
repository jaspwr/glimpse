#!/bin/bash

rm -rf ~/.cache/glimpse

sudo systemctl stop glimpse-monitor.service &&
sudo systemctl disable glimpse-monitor.service &&

sudo rm /etc/systemd/system/glimpse-monitor.service &&

sudo rm /usr/local/bin/glimpse &&
sudo rm /usr/local/bin/glimpse-indexer &&
sudo rm /usr/local/bin/glimpse-monitor &&

echo "Uninstalled."
