#!/bin/bash

# Stop on error:
set -e

initial_setup () {
    # Make sure bun installed as used in e.g. prettier scripts:
    if command -v bun > /dev/null 2>&1; then
        echo "bun already installed"
    else
        echo "bun could not be found, installing..."
        curl -fsSL https://bun.sh/install | bash # for macOS, Linux, and WSL
    fi

    # Make sure nightly is installed as needed for formatting in pre-commit:
    rustup toolchain install nightly
    # Make sure nextest is installed:
    cargo install cargo-nextest --locked

    # Make sure the prettier subdir package is all installed:
    cd ./prettier
    npm install
    cd ..

    # Install pre-commit if not already:
    pipx install pre-commit || true
    pre-commit install

    echo "Setting up docs..."
    cd docs
    # PDM_IGNORE_ACTIVE_VENV just in case running from inside a venv, don't want to use it:
    PDM_IGNORE_ACTIVE_VENV=True pdm install
    cd ..



    echo "Setting up rust backed python project..."
    ./dev_scripts/py_rust.sh ensure_venv



}

# Has to come at the end of these files:
source ./dev_scripts/_scr_setup/setup.sh "$@"
