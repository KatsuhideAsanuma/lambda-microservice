#!/bin/bash
set -e

echo "Installing wasm-pack..."

if ! command -v rustup &> /dev/null
then
    echo "Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source $HOME/.cargo/env
fi

curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh

echo "wasm-pack installation completed!"
