#!/usr/bin/env bash

set -e

# building wasm
cargo concordium build -e --out debug/mod.wasm

# init
cargo concordium run init \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--context debug/init_context.json \
--out-bin debug/state.bin 

# claim
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--context debug/receive_context.json \
--state-bin debug/state.bin \
--out-bin debug/state.bin \
--func "claim" 

# register
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--context debug/receive_context.json \
--state-bin debug/state.bin \
--out-bin debug/state.bin \
--func "register" \
--parameter-json parameters/register.json

# balanceExpiry
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--func "balanceExpiry" \
--state-bin debug/state.bin \
--parameter-json parameters/balanceExpiry.json \
--context debug/receive_context.json 

# lend
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--func "lend" \
--state-bin debug/state.bin \
--parameter-json parameters/lend.json \
--out-bin debug/state.bin \
--context debug/receive_context.json 

# transfer
cargo concordium run update \
--module debug/mod.wasm \
--contract "BictoryCnsNft" \
--func "transfer" \
--state-bin debug/state.bin \
--parameter-json parameters/transfer.json \
--context debug/receive_context.json

# # updateOperator
# cargo concordium run update \
# --module debug/mod.wasm \
# --contract "BictoryCnsNft" \
# --func "updateOperator" \
# --state-bin debug/state.bin \
# --parameter-json parameters/updateOperator.json \
# --context debug/receive_context.json

# # operatorOf
# cargo concordium run update \
# --module debug/mod.wasm \
# --contract "BictoryCnsNft" \
# --func "operatorOf" \
# --state-bin debug/state.bin \
# --parameter-json parameters/operatorOf.json \
# --context debug/receive_context.json
