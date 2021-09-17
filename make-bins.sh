#!/bin/bash
mkdir -p bin
for width in {2..15}; do
    for height in $(seq 2 $width); do
        SQUARE=$([ $width = $height ] && echo ",square")
        echo "building $width x $height $SQUARE"
        cargo build --release --no-default-features --features="width-$width,height-$height,unchecked,$SQUARE"
        cp target/release/fwrf bin/fwrf-${width}x${height}
    done
done