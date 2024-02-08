#!/bin/bash

# Stop on error:
set -e

update_versions() {
    echo "updating pkg versions across project..."



    # Sync the Cargo.toml py_rust version:
    zetch put ./py_rust/Cargo.toml package.version $(zetch var PY_RUST_VERSION)
    # Sync the release.yml workflow form:
    zetch put ./.github/workflows/release.yml on.workflow_dispatch.inputs.py_rust_version.default $(zetch var PY_RUST_VERSION)

}


# Has to come at the end of these files:
source ./dev_scripts/_scr_setup/setup.sh "$@"