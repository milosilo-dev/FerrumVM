#!/bin/bash
cargo build
sudo bash ./scripts/enable_tap.sh
sudo bash ./scripts/cargo-run.sh target/debug/ferrumvm
