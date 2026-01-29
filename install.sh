#!/bin/bash
set -e

cargo build --release
sudo cp target/release/maokai /usr/local/bin/maokai

echo "maokai installed to /usr/local/bin/maokai"
