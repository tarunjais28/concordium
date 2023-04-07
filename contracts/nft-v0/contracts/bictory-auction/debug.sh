#!/usr/bin/env bash

# building wasm
cargo concordium build -e --out debug/mod.wasm

# init
cargo concordium run init \
--module debug/mod.wasm \
--contract "BictoryAuction" \
--context debug/context.json \
--parameter-json parameters/init.json \
--out-bin debug/state.bin

# bid
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryAuction" \
--func "bid" \
--state-bin debug/state.bin \
--context debug/context.json \
--out-bin debug/state.bin \
--amount 1000000
