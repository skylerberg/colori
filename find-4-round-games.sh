#!/usr/bin/env bash
# Find game logs where the maximum round number is exactly 4.
# Usage: ./find-4-round-games.sh [game-logs-dir]

dir="${1:-game-logs}"

for f in "$dir"/*.json; do
  max_round=$(jq '[.entries[].round] | max' "$f")
  if [ "$max_round" -eq 4 ]; then
    echo "$f"
  fi
done
