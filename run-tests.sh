#!/bin/bash
cargo +nightly test || exit 1
cargo +nightly test --no-default-features --features=width-5,height-5,charset-english-small,square || exit 1
cargo +nightly test --no-default-features --features=width-6,height-4,charset-english-small || exit 1
echo
echo "All tests completed"