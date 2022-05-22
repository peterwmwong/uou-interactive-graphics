#!/bin/sh

set -e

CRATE_NAME=$1
CRATE_NAME_NORMALIZED=$(echo "$CRATE_NAME" | tr "-" "_")
BEFORE_ASM_PATH="/tmp/$CRATE_NAME-BEFORE.s"
AFTER_ASM_PATH="/tmp/$CRATE_NAME-AFTER.s"

RUSTFLAGS="--emit asm -C target-cpu=native" \
  cargo build --release -p $CRATE_NAME

touch $BEFORE_ASM_PATH
touch $AFTER_ASM_PATH
/usr/local/bin/code-insiders -r --diff $BEFORE_ASM_PATH $AFTER_ASM_PATH
cat ./target/release/deps/metal_app-*.s ./target/release/deps/"$CRATE_NAME_NORMALIZED"-*.s > $AFTER_ASM_PATH