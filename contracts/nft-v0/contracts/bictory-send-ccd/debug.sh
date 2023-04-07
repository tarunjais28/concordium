#!/usr/bin/env bash

# building wasm
cargo concordium build -e --out debug/mod.wasm

# init
cargo concordium run init \
--module debug/mod.wasm \
--contract "BictorySendCCD" \
--context debug/context.json \
--out-bin debug/state.bin

# send
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictorySendCCD" \
--func "send" \
--state-bin debug/state.bin \
--context debug/context.json \
--parameter-json parameters/send.json \
--amount 0.000001
