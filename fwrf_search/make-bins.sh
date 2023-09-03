#!/bin/bash
mkdir -p bin
for width in {2..15}; do
    for height in $(seq 2 $width); do
        SQUARE=$([ $width = $height ] && echo ",square")
        echo "building $width x $height $SQUARE"
        RUSTC_FLAGS="-C target-cpu=native" cargo +nightly build --release --no-default-features --features="width-$width,height-$height,unchecked,charset-english-extended$SQUARE" || exit 1
        cp target/release/fwrf bin/fwrf-${width}x${height}
    done
done