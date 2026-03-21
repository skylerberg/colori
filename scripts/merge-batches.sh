#!/usr/bin/env bash
set -euo pipefail

usage() {
    echo "Usage: $(basename "$0") [--dir DIR] BATCH_ID_1 BATCH_ID_2"
    echo ""
    echo "Merge two game log batches by renaming files from the older batch"
    echo "to use the newer batch's identifier."
    echo ""
    echo "Options:"
    echo "  --dir DIR   Directory containing game logs (default: game-logs/)"
    echo "  --help      Show this help message"
    exit "${1:-0}"
}

dir="game-logs"

while [[ $# -gt 0 ]]; do
    case "$1" in
        --dir)
            dir="$2"
            shift 2
            ;;
        --help|-h)
            usage 0
            ;;
        -*)
            echo "Unknown option: $1" >&2
            usage 1
            ;;
        *)
            break
            ;;
    esac
done

if [[ $# -ne 2 ]]; then
    echo "Error: expected exactly 2 batch IDs" >&2
    usage 1
fi

batch1="$1"
batch2="$2"

if [[ "$batch1" == "$batch2" ]]; then
    echo "Error: batch IDs are the same" >&2
    exit 1
fi

if [[ ! -d "$dir" ]]; then
    echo "Error: directory '$dir' does not exist" >&2
    exit 1
fi

# Find the minimum timestamp for a batch to determine which is older
min_timestamp() {
    local batch_id="$1"
    local min=""
    for f in "$dir"/game-*-"$batch_id"-*.json; do
        [[ -e "$f" ]] || continue
        local name
        name=$(basename "$f")
        local ts
        ts=$(echo "$name" | sed 's/game-\([0-9]*\)-.*/\1/')
        if [[ -z "$min" ]] || [[ "$ts" < "$min" ]]; then
            min="$ts"
        fi
    done
    echo "$min"
}

min1=$(min_timestamp "$batch1")
min2=$(min_timestamp "$batch2")

if [[ -z "$min1" ]]; then
    echo "Error: no game logs found for batch '$batch1' in $dir" >&2
    exit 1
fi

if [[ -z "$min2" ]]; then
    echo "Error: no game logs found for batch '$batch2' in $dir" >&2
    exit 1
fi

if [[ "$min1" < "$min2" ]]; then
    old_batch="$batch1"
    new_batch="$batch2"
else
    old_batch="$batch2"
    new_batch="$batch1"
fi

echo "Older batch: $old_batch -> Newer batch: $new_batch"

count=0
for f in "$dir"/game-*-"$old_batch"-*.json; do
    [[ -e "$f" ]] || continue
    new_name="${f/-$old_batch-/-$new_batch-}"
    mv "$f" "$new_name"
    count=$((count + 1))
done

echo "Renamed $count files"
