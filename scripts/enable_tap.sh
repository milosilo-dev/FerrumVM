#!/bin/bash
sudo ip link set ferrum-tap0 up
sudo ip addr add 10.0.2.1/24 dev ferrum-tap0