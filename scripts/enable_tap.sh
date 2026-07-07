#!/bin/bash
set -e

# Check if the tun driver is available (built-in or loadable)
TUN_AVAILABLE=0
if lsmod | grep -q "^tun"; then
    TUN_AVAILABLE=1
elif grep -q "^tun" /proc/modules 2>/dev/null; then
    TUN_AVAILABLE=1
elif [ -d "/lib/modules/$(uname -r)" ] && find "/lib/modules/$(uname -r)" -name "tun.ko*" -quit | grep -q .; then
    echo "Loading tun kernel module..."
    sudo modprobe tun
    TUN_AVAILABLE=1
elif grep -q "CONFIG_TUN=y" /proc/config.gz 2>/dev/null || grep -q "CONFIG_TUN=y" /boot/config-$(uname -r) 2>/dev/null; then
    # Built into the kernel, device should work
    TUN_AVAILABLE=1
fi

if [ "$TUN_AVAILABLE" -eq 0 ]; then
    echo "ERROR: TUN/TAP driver is not available for kernel $(uname -r)."
    echo "       No modules found in /lib/modules/$(uname -r)/"
    INSTALLED=$(pacman -Q linux 2>/dev/null | awk '{print $2}')
    RUNNING=$(uname -r)
    echo "       Installed: linux $INSTALLED"
    echo "       Running:   $RUNNING"
    if [ -n "$INSTALLED" ]; then
        echo ""
        echo "FIX: Reboot to load the newer kernel: sudo reboot"
    fi
    exit 1
fi

# Ensure /dev/net/tun exists and is accessible
if [ ! -c /dev/net/tun ]; then
    echo "Creating /dev/net/tun..."
    sudo mkdir -p /dev/net
    sudo mknod /dev/net/tun c 10 200
fi
sudo chmod 666 /dev/net/tun

# Create persistent tap interface
if ! ip link show ferrum-tap0 &>/dev/null; then
    sudo ip tuntap add mode tap name ferrum-tap0
fi
sudo ip link set dev ferrum-tap0 up txqueuelen 10000
sudo ip addr add 10.0.2.1/24 dev ferrum-tap0 2>/dev/null || true

echo "TAP interface ready."
