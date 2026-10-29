#!/usr/bin/env bash

set -uo pipefail

# Usage:
# ./compare-fens.sh IMAGE_FOLDER [CHUST_BINARY] [FP16_MODEL] [FP32_MODEL]

IMAGE_DIR="${1:-}"
CHUST_BIN="${2:-../target/release/chust}"
FP16_MODEL="${3:-../assets/models/300.onnx}"
FP32_MODEL="${4:-../assets/models/chess_detection.onnx}"

if [[ -z "$IMAGE_DIR" || ! -d "$IMAGE_DIR" ]]; then
    echo "Usage: $0 IMAGE_FOLDER [CHUST_BINARY] [FP16_MODEL] [FP32_MODEL]"
    exit 1
fi

for file in "$CHUST_BIN" "$FP16_MODEL" "$FP32_MODEL"; do
    if [[ ! -f "$file" ]]; then
        echo "Error: File not found: $file"
        exit 1
    fi
done

IMAGE_DIR="$(realpath "$IMAGE_DIR")"
CHUST_BIN="$(realpath "$CHUST_BIN")"
FP16_MODEL="$(realpath "$FP16_MODEL")"
FP32_MODEL="$(realpath "$FP32_MODEL")"

OUT_DIR="fen_comparison_$(date +%Y%m%d_%H%M%S)"
mkdir -p "$OUT_DIR/logs"

ALL_RESULTS="$OUT_DIR/all_results.tsv"
DIFFERENCES="$OUT_DIR/differences.txt"
SUMMARY="$OUT_DIR/summary.txt"

printf 'Image\tStatus\tFP16_FEN\tFP32_FEN\n' > "$ALL_RESULTS"

cat > "$DIFFERENCES" <<EOF
FEN DIFFERENCES
===============

FP16 model: $FP16_MODEL
FP32 model: $FP32_MODEL

EOF

total=0
same=0
different=0
fp16_errors=0
fp32_errors=0
both_errors=0

echo "Image folder: $IMAGE_DIR"
echo "FP16 model:   $FP16_MODEL"
echo "FP32 model:   $FP32_MODEL"
echo "Output:       $OUT_DIR"
echo

while IFS= read -r -d '' image; do
    ((total += 1))

    name="${image#"$IMAGE_DIR"/}"
    printf '[%d] %s\n' "$total" "$name"

    # Keep stdout and stderr separate so we can extract FEN reliably.
    fp16_out="$(mktemp)"
    fp16_err="$(mktemp)"
    fp32_out="$(mktemp)"
    fp32_err="$(mktemp)"

    if "$CHUST_BIN" process "$image" --model-path "$FP16_MODEL" \
        >"$fp16_out" 2>"$fp16_err"; then
        fp16_rc=0
    else
        fp16_rc=$?
    fi

    if "$CHUST_BIN" process "$image" --model-path "$FP32_MODEL" \
        >"$fp32_out" 2>"$fp32_err"; then
        fp32_rc=0
    else
        fp32_rc=$?
    fi

    fen16="$(sed -n 's/^FEN:[[:space:]]*//p' "$fp16_out" | tail -n 1)"
    fen32="$(sed -n 's/^FEN:[[:space:]]*//p' "$fp32_out" | tail -n 1)"

    # A successful process must also produce a FEN.
    valid16=1
    valid32=1
    [[ $fp16_rc -eq 0 && -n "$fen16" ]] || valid16=0
    [[ $fp32_rc -eq 0 && -n "$fen32" ]] || valid32=0

    if (( !valid16 && !valid32 )); then
        status="BOTH_ERROR"
        ((both_errors += 1))
        ((fp16_errors += 1))
        ((fp32_errors += 1))
    elif (( !valid16 )); then
        status="FP16_ERROR"
        ((fp16_errors += 1))
    elif (( !valid32 )); then
        status="FP32_ERROR"
        ((fp32_errors += 1))
    elif [[ "$fen16" == "$fen32" ]]; then
        status="SAME"
        ((same += 1))
    else
        status="DIFFERENT"
        ((different += 1))
    fi

    # Record every image, including errors and identical outputs.
    printf '%s\t%s\t%s\t%s\n' \
        "$name" "$status" "$fen16" "$fen32" >> "$ALL_RESULTS"

    if [[ "$status" == "DIFFERENT" ]]; then
        {
            printf '\n----------------------------------------\n'
            printf 'Image: %s\n' "$name"
            printf 'Status: DIFFERENT\n\n'
            printf 'FP16 model (%s):\n%s\n\n' \
                "$(basename "$FP16_MODEL")" "$fen16"
            printf 'FP32 model (%s):\n%s\n' \
                "$(basename "$FP32_MODEL")" "$fen32"
        } >> "$DIFFERENCES"
    fi

    # Preserve diagnostic output for failed images.
    if (( !valid16 || !valid32 )); then
        safe_name="$(printf '%s' "$name" | tr '/ ' '__')"
        log_prefix="$OUT_DIR/logs/${total}_${safe_name}"

        {
            printf 'Image: %s\n' "$name"
            printf 'FP16 exit code: %s\n' "$fp16_rc"
            printf 'FP32 exit code: %s\n' "$fp32_rc"
        } > "${log_prefix}.info"

        cp "$fp16_out" "${log_prefix}_fp16.stdout"
        cp "$fp16_err" "${log_prefix}_fp16.stderr"
        cp "$fp32_out" "${log_prefix}_fp32.stdout"
        cp "$fp32_err" "${log_prefix}_fp32.stderr"
    fi

    echo "    $status"

    rm -f "$fp16_out" "$fp16_err" "$fp32_out" "$fp32_err"

done < <(
    find "$IMAGE_DIR" -type f \
        \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \
           -o -iname '*.webp' -o -iname '*.bmp' \) -print0 |
        sort -z
)

if (( total == 0 )); then
    echo "No supported images found in: $IMAGE_DIR"
    exit 1
fi

if (( different == 0 )); then
    printf 'No FEN differences found.\n' >> "$DIFFERENCES"
fi

# Agreement percentage uses only images where both models succeeded.
comparable=$((same + different))
if (( comparable > 0 )); then
    agreement="$(awk -v s="$same" -v n="$comparable" \
        'BEGIN { printf "%.2f", s * 100 / n }')"
else
    agreement="N/A"
fi

{
    echo "FEN COMPARISON SUMMARY"
    echo "======================"
    echo "Images processed:       $total"
    echo "Identical FENs:          $same"
    echo "Different FENs:          $different"
    echo "FP16 errors/missing FEN: $fp16_errors"
    echo "FP32 errors/missing FEN: $fp32_errors"
    echo "Both models errored:     $both_errors"
    echo "Comparable images:       $comparable"
    echo "Agreement:               ${agreement}%"
    echo
    echo "FP16 model: $FP16_MODEL"
    echo "FP32 model: $FP32_MODEL"
    echo
    echo "All results:  $ALL_RESULTS"
    echo "Differences:  $DIFFERENCES"
    echo "Error logs:   $OUT_DIR/logs/"
} | tee "$SUMMARY"

echo
echo "Finished! Results saved in: $OUT_DIR"

