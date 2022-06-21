#!/bin/sh

set -e
CRATE_NAME=$1
OUTPUT_POSTFIX=$2
CRATE_NAME_NORMALIZED=$(echo "$CRATE_NAME" | tr "-" "_")
OUTPUT_PATH="/tmp/$CRATE_NAME-$OUTPUT_POSTFIX.js"

ADDITIONAL_CRATES="|metal"

get_function_list() {
    cargo asm --release --lib -p "$CRATE_NAME" --full-name |
        cut -d\" -f2,3 |
        egrep "^<?($CRATE_NAME_NORMALIZED$ADDITIONAL_CRATES)" |
        sed -r 's/\" [\[](\d*)/,\1/g' |
        sed -r 's/(\[|\])//g' |
        sort
}

set +e
FUNCTION_LIST=$(get_function_list)
set -e

echo "/*" > "$OUTPUT_PATH"
padding=$(printf '%0.1s' " "{1..9})
while IFS= read -r line; do
    func=$(echo "$line" | cut -d, -f1)
    size=$(echo "$line" | cut -d, -f2)
    printf "%s%s %s\n" "${padding:${#size}}" "$size" "$func" >> "$OUTPUT_PATH"
done <<< "$FUNCTION_LIST"
echo "*/" >> "$OUTPUT_PATH"


echo "export default {" >> "$OUTPUT_PATH"
while IFS= read -r line; do
    func=$(echo "$line" | cut -d, -f1)
    size=$(echo "$line" | cut -d, -f2)
    echo "/* $size */ \"$func\":\`" >> "$OUTPUT_PATH"
    cargo asm --release --lib -p "$CRATE_NAME" --full-name "$func" |
        tail -n +2 |                                       # Remove first line (function name, already printed above).
        egrep -v "^L(tmp|loh|BB)\d+" |                     # Remove lines that labels (diff noise reduction)
        sed 's/LBB\([0-9]\)*_\([0-9]\)*/LBB###/g' |        # Normalize all Branch Labels (diff noise reduction), ex. LBB123_1 -> LBB###
        sed 's/Lloh\([0-9]\)*/Lloh###/g' >> "$OUTPUT_PATH" # Normalize all Lloh Addresses (diff noise reduction), ex. Lloh123 -> Lloh123
    echo "\`," >> "$OUTPUT_PATH"
done <<< "$FUNCTION_LIST"
echo "}" >> "$OUTPUT_PATH"
