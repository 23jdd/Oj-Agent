#!/bin/sh
set -eu
# Run on a dedicated Linux worker, never with the host / as the execution rootfs.
rootfs_dir=${OJ_SANDBOX_ROOTFS:-/opt/oj-agent/rootfs}
case "$rootfs_dir" in /|/home|/root|/usr) echo 'Select a dedicated rootfs directory.' >&2; exit 1;; esac
apt-get update
DEBIAN_FRONTEND=noninteractive apt-get install -y curl ca-certificates build-essential python3 debootstrap pkg-config
if ! command -v cargo >/dev/null 2>&1; then
  curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs -o /tmp/oj-rustup.sh
  sh /tmp/oj-rustup.sh -y --profile minimal --default-toolchain stable
fi
if [ -f /root/.cargo/env ]; then . /root/.cargo/env; fi
if [ ! -x "$rootfs_dir/usr/bin/python3" ]; then
  debootstrap --variant=minbase --include=python3,g++,rustc,binutils noble "$rootfs_dir" https://archive.ubuntu.com/ubuntu
fi
mkdir -p /opt/oj-agent "$rootfs_dir/job"
cargo build -p oj-runtime --release
cp target/release/oj-runtime /opt/oj-agent/oj-runtime-bin
cat > /opt/oj-agent/oj-runtime <<'EOF'
#!/bin/sh
export OJ_SANDBOX_ROOTFS=/opt/oj-agent/rootfs
exec /opt/oj-agent/oj-runtime-bin
EOF
chmod 755 /opt/oj-agent/oj-runtime
printf 'Worker installed: /opt/oj-agent/oj-runtime\n'
