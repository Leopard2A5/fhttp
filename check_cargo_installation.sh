#!/usr/bin/env bash
set -euxo pipefail

EXPECTED_VERSION="$1"

apt-get update && apt-get install -y curl build-essential pkg-config libssl-dev

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
export PATH=$HOME/.cargo/bin:$PATH

cargo install fhttp

fhttp --version > version.txt
if [ "$(cat version.txt)" == "fhttp $EXPECTED_VERSION" ]
then
  echo "success!"
else
  echo "version number is wrong!"
  false
fi
