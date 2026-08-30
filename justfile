ios_app := "target/dx/ook-reader/debug/ios/OokReader.app"
ios_device_app := "target/dx/ook-reader/release/ios/OokReader.app"
bundle_id := "com.dimaportenko.ook-reader"

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

install-ios: boot-ios
    dx build --platform ios
    cp assets/icons/ios/*.png {{ios_app}}/
    xcrun simctl install booted {{ios_app}}

release-macos:
    dx bundle --bundle macos --release
    open target/dx/ook-reader/bundle/macos/macos

devices-ios:
    xcrun devicectl list devices

pick-device query="":
    #!/usr/bin/env bash
    set -euo pipefail
    json=$(mktemp)
    script=$(mktemp)
    trap 'rm -f "$json" "$script"' EXIT
    xcrun devicectl list devices --json-output "$json" > /dev/null
    cat > "$script" <<'PY'
    import json, os, sys

    devices = json.load(open(sys.argv[1]))["result"]["devices"]
    rows = [
        (
            d["hardwareProperties"]["udid"],
            d["deviceProperties"].get("name", "?"),
            d["hardwareProperties"].get("marketingName", "?"),
            d["connectionProperties"].get("tunnelState", "?"),
        )
        for d in devices
    ]

    query = os.environ.get("QUERY", "").strip().lower()
    if query:
        rows = [r for r in rows if any(query in f.lower() for f in r[:3])]
        if not rows:
            sys.exit("No paired device matches %r. Try `just devices-ios`." % query)

    if not rows:
        sys.exit("No paired devices. Connect one and trust this Mac.")

    if len(rows) > 1:
        for i, (_, name, model, state) in enumerate(rows, 1):
            print("  %d) %s - %s (%s)" % (i, name, model, state), file=sys.stderr)
        try:
            prompt = open("/dev/tty")
        except OSError:
            prompt = sys.stdin
        print("Device [1-%d]: " % len(rows), end="", file=sys.stderr, flush=True)
        answer = prompt.readline().strip()
        if not answer.isdigit() or not 1 <= int(answer) <= len(rows):
            sys.exit("Not a choice: %r" % answer)
        rows = [rows[int(answer) - 1]]

    udid, name, model, _ = rows[0]
    print("Installing to %s - %s" % (name, model), file=sys.stderr)
    print(udid)
    PY
    QUERY="{{query}}" python3 "$script" "$json"

release-ios query="":
    #!/usr/bin/env bash
    set -euo pipefail
    udid=$(just pick-device "{{query}}")
    dx build --platform ios --release --device "$udid"
    cp assets/icons/ios/*.png "{{ios_device_app}}/"
    profile=$(security cms -D -i "{{ios_device_app}}/embedded.mobileprovision")
    team=$(echo "$profile" | plutil -extract ApplicationIdentifierPrefix.0 raw -)
    valid=$(security find-identity -v -p codesigning)
    identity=""
    for i in $(seq 0 $(($(echo "$profile" | plutil -extract DeveloperCertificates raw -) - 1))); do
      sha=$(echo "$profile" | plutil -extract DeveloperCertificates.$i raw - | base64 -d \
        | openssl x509 -inform DER -noout -fingerprint -sha1 | cut -d= -f2 | tr -d :)
      if echo "$valid" | grep -q "$sha"; then identity=$sha; break; fi
    done
    if [ -z "$identity" ]; then
      echo "No valid keychain identity matches the profile dx embedded." >&2
      echo "Its certificates have all expired or been revoked - refresh the profile in Xcode." >&2
      exit 1
    fi
    entitlements=$(mktemp)
    codesign -d --entitlements - --xml "{{ios_device_app}}" > "$entitlements"
    plutil -replace application-identifier -string "$team.{{bundle_id}}" "$entitlements"
    plutil -replace keychain-access-groups -json "[\"$team.{{bundle_id}}\"]" "$entitlements"
    codesign --force --entitlements "$entitlements" --sign "$identity" "{{ios_device_app}}"
    xcrun devicectl device install app --device "$udid" "{{ios_device_app}}"
