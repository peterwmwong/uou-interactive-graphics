#!/bin/sh

set -e

case "$1" in
  Release) BUILD_PROFILE="release" ;;
  Debug) BUILD_PROFILE="dev" ;;
  *) BUILD_PROFILE="unknown" ;;
esac

~/.cargo/bin/cargo build --profile $BUILD_PROFILE -p $2

[ $BUILD_PROFILE == "release" ] && TARGET_DIR="release" || TARGET_DIR="debug"

rm target/$TARGET_DIR/app_launched_by_xcode
cp target/$TARGET_DIR/$2 target/$TARGET_DIR/app_launched_by_xcode

rm -f /tmp/tmp.entitlements
/usr/libexec/PlistBuddy -c 'Add :com.apple.security.get-task-allow bool true' /tmp/tmp.entitlements
codesign -s - --entitlements /tmp/tmp.entitlements -f target/$TARGET_DIR/app_launched_by_xcode
