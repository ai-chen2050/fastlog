#!/usr/bin/env bash

set -e

echo "Install TEE Dependency"
sudo dnf upgrade -y
sudo dnf install -y git tmux htop openssl-devel wget curl perl clang docker-24.0.5-1.amzn2023.0.3 aws-nitro-enclaves-cli aws-nitro-enclaves-cli-devel
sudo groupadd docker || true
sudo usermod -aG ne ec2-user
sudo usermod -aG docker ec2-user
sudo systemctl restart nitro-enclaves-allocator.service
sudo systemctl restart docker
newgrp docker
sudo systemctl enable --now nitro-enclaves-allocator.service
sudo systemctl enable --now docker

echo "Install rust buildtools"
if ! command -v rustc &> /dev/null; then
    echo "Rust is not installed. Installing Rust..."
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o rustup.sh
    sh rustup.sh -y
    source "$HOME/.cargo/env"
    rustc --version
else
    echo "Rust is already installed."
fi

rustup target add x86_64-unknown-linux-musl

echo "Install musl-gcc"
if ! command -v musl-gcc &> /dev/null; then
    echo "musl-gcc is not installed. Installing musl-gcc..."
    curl -LO https://www.musl-libc.org/releases/musl-1.2.2.tar.gz
    tar xf musl-1.2.2.tar.gz
    cd musl-1.2.2
    ./configure && make && sudo make install
    echo "PATH=\"/usr/local/musl/bin/:$PATH\"" >> ~/.bashrc
    # shellcheck disable=SC1090
    source ~/.bashrc
    cd ..
else
    echo "musl-gcc is already installed."
fi
