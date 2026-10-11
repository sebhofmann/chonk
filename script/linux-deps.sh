#!/usr/bin/env bash
# Installiert die Systembibliotheken, die gpui unter Debian/Ubuntu zum Bauen braucht.
set -euo pipefail

sudo apt-get update
sudo apt-get install -y --no-install-recommends \
  build-essential clang cmake pkg-config \
  libfontconfig1-dev libfreetype-dev \
  libwayland-dev libxkbcommon-dev libxkbcommon-x11-dev \
  libx11-dev libx11-xcb-dev libxcb1-dev \
  libssl-dev libzstd-dev
