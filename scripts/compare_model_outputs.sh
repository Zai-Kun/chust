#!/usr/bin/env bash

set -uo pipefail

# Usage:
# ./compare-model-outputs.sh IMAGE_FOLDER [CHUST_BINARY] [MODEL_A] [MODEL_B] [LABEL_A] [LABEL_B] [OUTPUT_DIR]
#
# Defaults match the current Chust setup:
#   MODEL_A = chess_detection.onnx       (INT8, based on the current setup)
#   MODEL_B = chess_detectionfp16.onnx   (FP16)
#   Outputs = int8ss1.png, f16ss1.png, int8ss2.png, f16ss2.png, ...
#
# For a genuine FP32 model, pass its path as MODEL_A and use "fp32" as LABEL_A.

IMAGE_DIR="${1:-}"
CHUST_BIN="${2:-../target/release/chust}"
MODEL_A="${3:-../assets/models/chess_detection.onnx}"
MODEL_B="${4:-../assets/models/300.onnx}"
LABEL_A="${5:-int8}"
LABEL_B="${6:-f16}"
OUT_DIR="${7:-model_outputs_$(date +%Y%m%d_%H%M%S)}"

usage() {
    echo "Usage: $0 IMAGE_FOLDER [CHUST_BINARY] [MODEL_A] [MODEL_B] [LABEL_A] [LABEL_B] [OUTPUT_DIR]"
    echo "Example: $0 screenshots ./target/release/chust ./fp32.onnx ./chess_detectionfp16.onnx fp32 f16"
}

if [[ -z "$IMAGE_DIR" || ! -d "$IMAGE_DIR" ]]; then
    usage
    exit 1
fi

for file in "$CHUST_BIN" "$MODEL_A" "$MODEL_B"; do
    if [[ ! -f "$file" ]]; then
        echo "Error: File not found: $file" >&2
        exit 1
    fi
done

# Prevent labels from accidentally creating nested paths or overwriting unrelated files.
for label in "$LABEL_A" "$LABEL_B"; do
    if [[ ! "$label" =~ ^[[:alnum:]_-]+$ ]]; then
        echo "Error: Labels may contain only letters, numbers, underscores, and hyphens: $label" >&2
        exit 1
    fi
done

IMAGE_DIR="$(realpath "$IMAGE_DIR")"
CHUST_BIN="$(realpath "$CHUST_BIN")"
MODEL_A="$(realpath "$MODEL_A")"
MODEL_B="$(realpath "$MODEL_B")"
mkdir -p "$OUT_DIR/logs"
OUT_DIR="$(realpath "$OUT_DIR")"

ALL_RESULTS="$OUT_DIR/results.tsv"
printf 'Index\tSource_Image\t%s_Status\t%s_Output\t%s_Status\t%s_Output\n' \
    "$LABEL_A" "$LABEL_A" "$LABEL_B" "$LABEL_B" > "$ALL_RESULTS"

run_model() {
    local model="$1"
    local label="$2"
    local image="$3"
    local index="$4"
    local output_path="$OUT_DIR/${label}ss${index}.png"
    local log_prefix="$OUT_DIR/logs/${label}ss${index}"
    local rc

    # Remove a stale output if the user supplied an existing output directory.
    rm -f "$output_path"

    if "$CHUST_BIN" process "$image" \
        "--output-path=$output_path" \
        --model-path "$model" \
        >"${log_prefix}.stdout" 2>"${log_prefix}.stderr"; then
        rc=0
    else
        rc=$?
    fi

    if [[ $rc -eq 0 && -f "$output_path" ]]; then
        printf 'OK\t%s\t%s\n' "$output_path" "$log_prefix"
    else
        printf 'ERROR(rc=%s)\t%s\t%s\n' "$rc" "$output_path" "$log_prefix"
    fi
}

total=0
success_a=0
success_b=0
failed_a=0
failed_b=0

echo "Image folder: $IMAGE_DIR"
echo "Model A ($LABEL_A): $MODEL_A"
echo "Model B ($LABEL_B): $MODEL_B"
echo "Output directory: $OUT_DIR"
echo

while IFS= read -r -d '' image; do
    ((total += 1))
    name="${image#"$IMAGE_DIR"/}"
    printf '[%d] %s\n' "$total" "$name"

    result_a="$(run_model "$MODEL_A" "$LABEL_A" "$image" "$total")"
    IFS=$'\t' read -r status_a output_a log_a <<< "$result_a"
    printf '    %s: %s\n' "$LABEL_A" "$status_a"
    if [[ "$status_a" == "OK" ]]; then
        ((success_a += 1))
    else
        ((failed_a += 1))
    fi

    result_b="$(run_model "$MODEL_B" "$LABEL_B" "$image" "$total")"
    IFS=$'\t' read -r status_b output_b log_b <<< "$result_b"
    printf '    %s: %s\n' "$LABEL_B" "$status_b"
    if [[ "$status_b" == "OK" ]]; then
        ((success_b += 1))
    else
        ((failed_b += 1))
    fi

    printf '%s\t%s\t%s\t%s\t%s\t%s\n' \
        "$total" "$name" "$status_a" "$output_a" "$status_b" "$output_b" >> "$ALL_RESULTS"
done < <(
    find "$IMAGE_DIR" -type f \
        \( -iname '*.png' -o -iname '*.jpg' -o -iname '*.jpeg' \
           -o -iname '*.webp' -o -iname '*.bmp' \) -print0 |
        sort -z
)

if (( total == 0 )); then
    echo "No supported images found in: $IMAGE_DIR" >&2
    exit 1
fi

printf '\nFinished. Images processed: %d\n' "$total"
printf '%s successful: %d; failed: %d\n' "$LABEL_A" "$success_a" "$failed_a"
printf '%s successful: %d; failed: %d\n' "$LABEL_B" "$success_b" "$failed_b"
echo "Rendered images: $OUT_DIR/"
echo "Run logs:       $OUT_DIR/logs/"
echo "Results TSV:    $ALL_RESULTS"

if (( failed_a > 0 || failed_b > 0 )); then
    exit 2
fi
