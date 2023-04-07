#!/usr/bin/env bash

set -e

# building wasm
cargo concordium build -e --out debug/mod.wasm

# init
cargo concordium run init \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--context debug/init_context.json \
--out-bin debug/state.bin

# mint
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "mint" \
--state-bin debug/state.bin \
--parameter-json parameters/mint.json \
--context debug/receive_context.json \
--out-bin debug/state.bin \
--amount 0

# transfer
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "transfer" \
--state-bin debug/state.bin \
--parameter-json parameters/transfer.json \
--context debug/receive_context.json

# updateOperator
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "updateOperator" \
--state-bin debug/state.bin \
--parameter-json parameters/updateOperator.json \
--context debug/receive_context.json

# operatorOf
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "operatorOf" \
--state-bin debug/state.bin \
--parameter-json parameters/operatorOf.json \
--context debug/receive_context.json

# burn
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "burn" \
--state-bin debug/state.bin \
--parameter-json parameters/burn.json \
--context debug/receive_context.json

# updatePrice
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryNFT" \
--func "updatePrice" \
--state-bin debug/state.bin \
--parameter-json parameters/updatePrice.json \
--context debug/receive_context.json
