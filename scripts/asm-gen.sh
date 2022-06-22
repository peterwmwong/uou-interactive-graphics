#!/bin/sh

set -e
CRATE_NAME=$1
OUTPUT_POSTFIX=$2
CRATE_NAME_NORMALIZED=$(echo "$CRATE_NAME" | tr "-" "_")
TMP_OUTPUT_PATH="/tmp/$CRATE_NAME-$OUTPUT_POSTFIX-TMP.js"
OUTPUT_PATH="/tmp/$CRATE_NAME-$OUTPUT_POSTFIX.js"

ADDITIONAL_CRATES="|metal"
DEFAULT_CARGO_ASM_ARGS="asm --release --lib --full-name -p $CRATE_NAME"

get_function_list() {
    cargo $DEFAULT_CARGO_ASM_ARGS |
        cut -d\" -f2,3 |
        egrep "^<?($CRATE_NAME_NORMALIZED$ADDITIONAL_CRATES)" |
        sed -r 's/\" [\[](\d*)/\|\1/g' |
        sed -r 's/(\[|\])//g' |
        sort
}

set +e
FUNCTION_LIST=$(get_function_list)
set -e

i=0
while IFS= read -r line; do
    func=$(echo "$line" | cut -d\| -f1)
    size=$(echo "$line" | cut -d\| -f2)
    echo "/* $size */ \"$func\":\`" > "$TMP_OUTPUT_PATH$i"
    cargo $DEFAULT_CARGO_ASM_ARGS "$func" |
        tail -n +2 |                                               # Remove first line (function name, already printed above).
        egrep -v "^L(tmp|loh|BB)\d+" |                             # Remove lines that labels (diff noise reduction)
        sed 's/LBB\([0-9]\)*_\([0-9]\)*/LBB###/g' |                # Normalize all Branch Labels (diff noise reduction), ex. LBB123_1 -> LBB###
        sed 's/Lloh\([0-9]\)*/Lloh###/g' >> "$TMP_OUTPUT_PATH$i" & # Normalize all Lloh Addresses (diff noise reduction), ex. Lloh123 -> Lloh123
    pids[${i}]=$!
    i=$((i+1))
done <<< "$FUNCTION_LIST"

echo "/*" > "$TMP_OUTPUT_PATH"
padding=$(printf '%0.1s' " "{1..9})
while IFS= read -r line; do
    func=$(echo "$line" | cut -d\| -f1)
    size=$(echo "$line" | cut -d\| -f2)
    printf "%s%s %s\n" "${padding:${#size}}" "$size" "$func" >> "$TMP_OUTPUT_PATH"
done <<< "$FUNCTION_LIST"
echo "*/" >> "$TMP_OUTPUT_PATH"

echo "export default {" >> "$TMP_OUTPUT_PATH"
for i in ${!pids[@]}; do
    wait "${pids[$i]}"
    cat "$TMP_OUTPUT_PATH$i" >> "$TMP_OUTPUT_PATH"
    echo "\`," >> "$TMP_OUTPUT_PATH"
done
echo "}" >> "$TMP_OUTPUT_PATH"

mv "$TMP_OUTPUT_PATH" "$OUTPUT_PATH"
rm "$TMP_OUTPUT_PATH"*
