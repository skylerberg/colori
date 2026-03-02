#!/usr/bin/env bash
# Move all but the most recent batch of games from game-logs to game-log-archive.
# Batches are identified by the suffix in the filename: game-<timestamp>-<batch>.json
# The most recent batch is determined by the highest timestamp among its files.
# Usage: ./archive-old-games.sh [game-logs-dir]

set -euo pipefail

dir="${1:-game-logs}"
archive_dir="game-log-archive"

# Find the most recent batch by getting the last file (highest timestamp) and extracting its suffix.
latest_file=$(ls "$dir"/*.json | sort | tail -1)
latest_batch=$(echo "$latest_file" | sed 's/.*-\([^.]*\)\.json/\1/')

echo "Most recent batch: $latest_batch"

# Collect files that don't belong to the latest batch.
files_to_move=()
for f in "$dir"/*.json; do
  batch=$(echo "$f" | sed 's/.*-\([^.]*\)\.json/\1/')
  if [ "$batch" != "$latest_batch" ]; then
    files_to_move+=("$f")
  fi
done

if [ ${#files_to_move[@]} -eq 0 ]; then
  echo "No older batches to archive."
  exit 0
fi

echo "Archiving ${#files_to_move[@]} games to $archive_dir/"
mkdir -p "$archive_dir"
mv "${files_to_move[@]}" "$archive_dir/"
echo "Done."
