#!/bin/bash

# Stop on error:
set -e

# Prep for running top-level services
_prep () {
    # A custom env version may have been used before, reset zetch to make sure not the case.
    zetch



}



# Has to come at the end of these files:
source ./dev_scripts/_scr_setup/setup.sh "$@"