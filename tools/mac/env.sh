# Settings for building Mimi on a Mac. Use it from the project folder, before cargo or tauri:
#   . tools/mac/env.sh

# .cargo/config.toml names a C: folder for Windows' path limit. This takes its place on a Mac.
export CARGO_TARGET_DIR="$HOME/code/mimi-build"

# Software updates can leave a macOS SDK newer than the compiler tools, and the linker then
# fails with "tapi error: malformed file". Use the SDK that matches the tools, when there is one.
tools_major=$(pkgutil --pkg-info=com.apple.pkg.CLTools_Executables 2>/dev/null | awk '/^version:/ { split($2, part, "."); print part[1] }')
matching_sdk="/Library/Developer/CommandLineTools/SDKs/MacOSX${tools_major}.sdk"
if [ -n "$tools_major" ] && [ -d "$matching_sdk" ]; then
  export SDKROOT="$matching_sdk"
fi
unset tools_major matching_sdk

# Signing with one certificate lets macOS keep Mimi's permissions between builds.
# Made once by tools/mac/create_signing_certificate.sh. Without it, builds are signed ad hoc.
if security find-certificate -c "Mimi Local Signing" "$HOME/Library/Keychains/login.keychain-db" >/dev/null 2>&1; then
  export APPLE_SIGNING_IDENTITY="Mimi Local Signing"
fi
