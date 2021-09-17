#!/bin/bash
for width in {2..15}; do
    for height in $(seq 2 $width); do
        bin/fwrf-${width}x${height} -q --ignore-empty-wordlist "$@"
    done
done