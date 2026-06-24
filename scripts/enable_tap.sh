#!/bin/bash
if ! ip link show ferrum-tap0 &>/dev/null; then
    sudo ip tuntap add mode tap name ferrum-tap0
fi
sudo ip link set ferrum-tap0 up
sudo ip addr add 10.0.2.1/24 dev ferrum-tap0 2>/dev/null || true