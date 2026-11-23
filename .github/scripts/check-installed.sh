#!/usr/bin/env bash
set -euo pipefail

unset PRESS_LIB_DIR PRESS_LIB PKG_CONFIG_PATH LD_LIBRARY_PATH PRESS_STYLE_PATH
version=$(sed -n 's/^version *= *"\([^"]*\)"/\1/p' Cargo.toml)
test "$(pkg-config --modversion press)" = "$version"

swift test

work=$(mktemp -d)
git_server=
cleanup() {
    if [[ -n $git_server ]]; then
        kill "$git_server" 2>/dev/null || true
        wait "$git_server" 2>/dev/null || true
    fi
    rm -rf "$work"
}
trap cleanup EXIT

# Use a versioned Git dependency without adding a tag to the CI checkout.
git init -q --bare --initial-branch=main "$work/press.git"
git --git-dir="$work/press.git" fetch --quiet --no-tags "$PWD" HEAD:refs/heads/main
git --git-dir="$work/press.git" tag "v$version" refs/heads/main
git daemon --reuseaddr --export-all --base-path="$work" --listen=127.0.0.1 --port=9418 "$work" &
git_server=$!
url=git://127.0.0.1:9418/press.git
for attempt in {1..10}; do
    if git ls-remote --exit-code "$url" "refs/tags/v$version" >/dev/null 2>&1; then
        break
    fi
    sleep 1
done
git ls-remote --exit-code "$url" "refs/tags/v$version" >/dev/null

cc examples/c/hello.c $(pkg-config --cflags --libs press) -o "$work/c-hello"
export PRESS_BACKEND=headless
PRESS_STYLE=eightyfour "$work/c-hello" --snapshot "$work/builtin"
PRESS_STYLE=ninetyfive "$work/c-hello" --snapshot "$work/plugin" 2>"$work/plugin.log"
PRESS_STYLE=ninetyfive-desert "$work/c-hello" --snapshot "$work/theme" 2>"$work/theme.log"
test -s "$work/builtin-1.png"
test -s "$work/plugin-1.png"
test -s "$work/theme-1.png"
if grep -E 'not in use|could not be loaded' "$work/plugin.log" "$work/theme.log" ||
   cmp -s "$work/builtin-1.png" "$work/plugin-1.png" ||
   cmp -s "$work/plugin-1.png" "$work/theme-1.png"; then
    echo "The installed style or theme failed to render." >&2
    exit 1
fi

mkdir -p "$work/consumer/Sources/Hello" "$work/consumer/Sources/CheckCore"
cd "$work/consumer"
cat > Package.swift <<SWIFT
// swift-tools-version:5.9
import PackageDescription

let package = Package(
    name: "PressConsumer",
    dependencies: [.package(url: "$url", exact: "$version")],
    targets: [
        .executableTarget(name: "Hello", dependencies: [.product(name: "Press", package: "press")]),
        .executableTarget(name: "CheckCore", dependencies: [.product(name: "PressCore", package: "press")]),
    ]
)
SWIFT
cat > Sources/Hello/Hello.swift <<'SWIFT'
import Press

@main
struct Hello: App {
    var body: some Scene {
        WindowGroup("Hello") {
            Text("Hello from PRESS.").padding(16)
        }
    }
}
SWIFT
cat > Sources/CheckCore/main.swift <<'SWIFT'
import PressCore

try App.initialize()
let version = try App.abiVersion()
precondition(version == pressABIVersion)
try App.shutdown()
print("PRESS ABI \(version)")
SWIFT

swift build
binary_dir=$(swift build --show-bin-path)
ldd "$binary_dir/Hello" | grep -E 'libpress\.so\.1 => /(usr/)?lib/'
"$binary_dir/CheckCore"
PRESS_SNAPSHOT="$work/swift-hello.png" "$binary_dir/Hello"
test -s "$work/swift-hello.png"
