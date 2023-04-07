#!/usr/bin/env bash

# building wasm
cargo concordium build -e --out debug/mod.wasm

# init
cargo concordium run init \
--module debug/mod.wasm \
--contract "BictoryConfig" \
--context debug/context.json \
--out-bin debug/state.bin \
--parameter-json parameters/royalty.json

# # updating royalty
# cargo concordium run update \
# --module debug/mod.wasm \
# --contract "BictoryConfig" \
# --func "updateRoyalty" \
# --state-bin debug/state.bin \
# --parameter-json parameters/royalty.json \
# --context debug/context.json
