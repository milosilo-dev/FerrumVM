#!/bin/bash
if ip link show ferrum-tap0 &>/dev/null; then
    sudo ip link set ferrum-tap0 down
    sudo ip tuntap del mode tap name ferrum-tap0
fi
