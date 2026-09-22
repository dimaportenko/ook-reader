ios_app := "target/dx/ook-reader/debug/ios/OokReader.app"
ios_device_app := "target/dx/ook-reader/release/ios/OokReader.app"
bundle_id := "com.dimaportenko.ook-reader"

# Dioxus 0.7 pins tao 0.34, which has no UIScene life cycle; the iOS 27 SDK (Xcode 27)
# refuses to launch such apps. Until Dioxus bumps tao (DioxusLabs/dioxus#5853), iOS
# builds link against the iOS 26 SDK from a side-by-side Xcode 26 install.
ios_xcode := env("OOK_IOS_XCODE", "/Applications/Xcode-26.6.0.app")

default:
    @just --list

test:
    RUST_BACKTRACE=1 cargo test

validate:
    cargo test && cargo clippy --all-targets

serve-desktop:
    #!/usr/bin/env bash
    set -euo pipefail
    entitlements=$(mktemp)
    trap 'rm -f "$entitlements"' EXIT
    plutil -create xml1 "$entitlements"
    dx serve --platform desktop --codesign --apple-team-id 70478F49E6BA83927FF78D2C6F8C7A66D1F7C5CF --apple-entitlements "$entitlements"

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

check-ios-xcode:
    #!/usr/bin/env bash
    set -euo pipefail
    if [ ! -d "{{ios_xcode}}/Contents/Developer" ]; then
      echo "Xcode 26 not found at {{ios_xcode}}." >&2
      echo "Install it side by side (e.g. via Xcodes.app) or set OOK_IOS_XCODE to its path." >&2
      echo "Why: Xcode 27's iOS SDK requires UIScene, which Dioxus 0.7's tao 0.34 lacks (DioxusLabs/dioxus#5853)." >&2
      exit 1
    fi
    echo "Using $(/usr/libexec/PlistBuddy -c 'Print CFBundleShortVersionString' "{{ios_xcode}}/Contents/Info.plist" | sed 's/^/Xcode /') at {{ios_xcode}}"

serve-ios: check-ios-xcode boot-ios
    DEVELOPER_DIR="{{ios_xcode}}/Contents/Developer" dx serve --platform ios

install-ios: check-ios-xcode boot-ios
    DEVELOPER_DIR="{{ios_xcode}}/Contents/Developer" dx build --platform ios
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

release-ios query="": check-ios-xcode
    #!/usr/bin/env bash
    set -euo pipefail
    export DEVELOPER_DIR="{{ios_xcode}}/Contents/Developer"
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
