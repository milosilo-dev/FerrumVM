mount -a
apk add nix git
cd /mnt
git config --global --add safe.directory /mnt
nix --extra-experimental-features 'nix-command flakes' develop