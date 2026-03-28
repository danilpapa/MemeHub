#!/bin/bash

set -euo pipefail

install_homebrew() {
    if ! command -v brew &> /dev/null; then
        echo "Устанавливаем Homebrew..."
        
        /bin/bash -c "$(curl -fsSL https://raw.githubusercontent.com/Homebrew/install/HEAD/install.sh)" || {
            echo "Ошибка при установке Homebrew. Прерывание."
            exit 1
        }
        
        if [[ $(uname -m) == "arm64" ]]; then
            BREW_PATH="/opt/homebrew/bin/brew"
        else
            BREW_PATH="/usr/local/bin/brew"
        fi

        eval "$($BREW_PATH shellenv)"
        echo 'eval "$('$BREW_PATH' shellenv)"' >> ~/.zprofile
    else
        echo "Homebrew уже установлен"
    fi
}

install_rust() {
    if ! command -v rustup &> /dev/null; then
        echo "Устанавливаем Rust через Homebrew..."
        brew install rustup-init
        
        rustup-init -y
        
        if [[ -f "$HOME/.cargo/env" ]]; then
            source "$HOME/.cargo/env"
            
            if ! grep -q '.cargo/bin' ~/.zshrc; then
                echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
                echo "Добавили ~/.cargo/bin в PATH"
            fi
        fi
    else
            if [[ -f "$HOME/.cargo/env" ]]; then
            source "$HOME/.cargo/env"
            
            if ! grep -q '.cargo/bin' ~/.zshrc; then
                echo 'export PATH="$HOME/.cargo/bin:$PATH"' >> ~/.zshrc
                echo "Добавили ~/.cargo/bin в PATH"
            fi
        fi
        echo "rustup уже установлен"
    fi
}

install_servicectl() {
    cargo install --git https://github.com/danilpapa/servicectl --force
}

main() {
    install_homebrew
    install_rust
    install_servicectl
}

main