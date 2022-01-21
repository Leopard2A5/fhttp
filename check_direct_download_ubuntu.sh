#!/usr/bin/env bash
set -euxo pipefail

EXPECTED_VERSION="$1"

apt-get update && apt-get install -y curl

sleep 60

curl -LJO https://github.com/Leopard2A5/fhttp/releases/download/$EXPECTED_VERSION/fhttp.linux_x64

ls -la

chmod +x fhttp.linux_x64

./fhttp.linux_x64 --version > version.txt
if [ "$(cat version.txt)" == "fhttp $EXPECTED_VERSION" ]
then
  echo "success!"
else
  echo "version number is wrong!"
  false
fi
