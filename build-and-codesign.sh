#!/bin/sh

set -e

~/.cargo/bin/cargo build --profile $1

[ $1 == "release" ] && TARGET_DIR="release" || TARGET_DIR="debug"

rm -f /tmp/tmp.entitlements
/usr/libexec/PlistBuddy -c 'Add :com.apple.security.get-task-allow bool true' /tmp/tmp.entitlements
codesign -s - --entitlements /tmp/tmp.entitlements -f target/$TARGET_DIR/uou-interactive-graphics
