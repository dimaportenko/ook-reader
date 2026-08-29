default:
    @just --list

test:
    RUST_BACKTRACE=1 cargo test

validate:
    cargo test && cargo clippy --all-targets

serve-desktop:
    dx serve --platform desktop

boot-ios:
    #!/usr/bin/env bash
    set -euo pipefail
    DEVICE="${DEVICE:-$(xcrun simctl list devices available | awk '
      /iPhone/ && match($0, /[0-9A-F]+-[0-9A-F]+-[0-9A-F]+-[0-9A-F]+-[0-9A-F]+/) {
        udid = substr($0, RSTART, RLENGTH)
        if ($0 ~ /\(Booted\)/) booted = udid
        latest = udid
      }
      END { print (booted != "" ? booted : latest) }
    ')}"
    if [ -z "$DEVICE" ]; then
      echo "No available iPhone simulator found. Install one via Xcode > Settings > Components." >&2
      exit 1
    fi
    echo "Using simulator: $(xcrun simctl list devices | grep -i "$DEVICE" | head -1 | sed 's/^ *//')"
    xcrun simctl boot "$DEVICE" 2>/dev/null || true
    open -a Simulator
    xcrun simctl bootstatus "$DEVICE" -b

serve-ios: boot-ios
    dx serve --platform ios

bundle-macos:
    dx bundle --bundle macos --release
