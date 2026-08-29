#!/bin/sh
set -eu

for _ in $(seq 1 50); do
    count=$(niri msg -j windows | jq '[.[] | select(.app_id == "fixture-plane")] | length')
    [ "$count" -eq 9 ] && break
    sleep 0.1
done

focus() {
    id=$(niri msg -j windows | jq -r --arg title "$1" '.[] | select(.title == $title) | .id' | head -n1)
    niri msg action focus-window --id "$id"
}

focus W3
niri msg action move-window-down
focus W6
niri msg action move-window-down
focus W8
niri msg action move-window-up
focus W1
