#!/usr/bin/env bash

case "$1" in
  build)
    cargo concordium build -e --out debug/mod.wasm
    ;;

  init)
    cargo concordium build -e --out debug/mod.wasm

    touch debug/state.bin

    cargo concordium run init \
      --module debug/mod.wasm \
      --contract "BictoryStorage" \
      --context debug/init_context.json \
      --out-bin debug/state.bin
    ;;

  update)
    cargo concordium run update \
      --module "debug/mod.wasm" \
      --contract "BictoryStorage" \
      --entrypoint "$2" \
      --state-bin "debug/state.bin" \
      --parameter-json "parameters/${2}.json" \
      --context "debug/receive_context.json" \
      --out-bin "debug/state.bin"
    ;;

  *)
    echo "Unknown option used. Available options: init, update"
    ;;
esac
