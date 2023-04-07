#!/usr/bin/env bash

DOMAINS_FILE="$1"
OUT_FILE="./outcome.txt"

# Configuration
CCD_IP="135.181.99.124"
CCD_PORT="10000"
SENDER="Joha-CNS"
CONTRACT="7076"
ADDRESS='3BtQiJ1sNad8cp5JLdBrjuRYkoX6A8fanzHpoSshKEX6ghAWwi'
DURATION="1"

function echoerr() {
  echo "$@" >&2
}

if ! [ -f "$DOMAINS_FILE" ]; then
  echoerr "Error: Domain file can't be found or read"
  exit 1
fi

if [ -f "$OUT_FILE" ]; then
  read -p "Outcome file exists. Overwrite? [y/N] " -n 1 -r
  echoerr
  if [ "$REPLY" = "y" ] || [ "$REPLY" = "Y" ]; then
    if ! printf "" > "$OUT_FILE"; then
      echoerr "Error: Unable to create output file, aborting."
      exit 1
    fi
  else
    echoerr "Error: output file already exists."
    exit 1
  fi
fi

function echoout() {
  echo "$@" | tee -a "$OUT_FILE"
}

# Register parameters template
read -r -d '' PARAMS << EOF
{
    "domain": ::DOMAIN::,
    "address": {
        "Account": [
            ::ADDRESS::
        ]
    },
    "duration_years": ::DURATION::
}
EOF

# Temporary directory for parameters and output
TEMP_DIR=$(mktemp -d)
echoerr "Created a temporary directory: $TEMP_DIR"

mkdir "$TEMP_DIR/params"
mkfifo "$TEMP_DIR/in"
touch "$TEMP_DIR/out"

declare -A DOMAIN_TX

function cleanup() {
  echoerr
  echoerr "Cleaning up temporary directory..."

  rm -r "$TEMP_DIR" && echoerr "Done!" || echoerr "Failed!"

  if [ "${#DOMAIN_TX[@]}" -gt "0" ]; then
    for domain in "${!DOMAIN_TX[@]}"; do
      echoout "unknown ${DOMAIN_TX[$domain]} $domain"
    done
  fi

  exit
}

trap cleanup EXIT

echoerr
IFS= read -p "Password for $SENDER: " -rs PASSWORD
echoerr

function get_nonce() {
  concordium-client --grpc-ip "$CCD_IP" --grpc-port "$CCD_PORT" account show "$SENDER" | awk '/Nonce:/ { print $2 }'
}

# Send a tx and write stderr output to $TEMP_DIR/out
function send_tx() {
  domain="$1"
  address="$2"
  nonce="$3"

  domain_string="\"$domain\""
  address_string="\"$address\""

  echo $PARAMS | awk -v domain="$domain_string" -v address="$address_string" -v duration="$DURATION" \
    '{ gsub("::DOMAIN::",domain); gsub("::ADDRESS::",address); gsub("::DURATION::",duration) }1' > "$TEMP_DIR/params/$domain"

  echo "y" >"$TEMP_DIR/in" && echo "$PASSWORD" >"$TEMP_DIR/in" &

  concordium-client --grpc-ip "$CCD_IP" --grpc-port "$CCD_PORT" contract update "$CONTRACT" \
    --energy 20000 --entrypoint "register" --schema "parameters/schema.bin" --parameter-json "$TEMP_DIR/params/$domain" --sender "$SENDER" --nonce "$nonce" --no-wait \
    <"$TEMP_DIR/in" >/dev/null 2>"$TEMP_DIR/out" &

  pid="$!"

  wait $pid
}

echoerr "Transaction results will be saved to '$OUT_FILE'"

nonce=$(get_nonce)

while IFS="" read -r domain || [ -n "$domain" ]; do
  fails=0
  while true; do
    send_tx "$domain" "$ADDRESS" "$nonce"

    outcome=$(tail -1 "$TEMP_DIR/out")

    case "$outcome" in
      "concordium-client: user error (cannot decrypt signing key with index 0: decryption failure: wrong password)"*)
        echoerr "Password error"
        exit 1
        ;;

      "concordium-client: user error (gRPC error: Duplicate nonce)"*)
        echoerr "Warning: Duplicate nonce"
        echoerr "Waiting for block finalization to query nonce again..."
        sleep 10
        nonce=$(get_nonce)
        ;;

      "Transaction '"*"' sent to the baker"*)
        nonce=$((nonce + 1))

        tx_hash=${outcome#"Transaction '"}
        tx_hash=${tx_hash%"' sent to the baker"*}

        if [ "$tx_hash" != "" ]; then
          echoerr "Register tx sent for domain '$domain': $tx_hash"
          DOMAIN_TX["$domain"]="$tx_hash"
        else
          # TODO: What do I do?..
          echoerr "Error: Unable to read tx hash for domain '$domain'"
        fi

        break
        ;;

      *)
        echoerr "Unknown error: $outcome"
        fails=$((fails + 1))
        if [ "$fails" -gt 5 ]; then
          echoerr "Error: 5 fails in a row, aborting..."
          exit 1
        fi
        ;;
    esac
  done
done < "$DOMAINS_FILE"

for domain in "${!DOMAIN_TX[@]}"; do
  tx_hash="${DOMAIN_TX[$domain]}"
  fails=0
  while true; do
    concordium-client --grpc-ip "$CCD_IP" --grpc-port "$CCD_PORT" transaction status "$tx_hash" >"$TEMP_DIR/out"

    outcome=$(sed '1q;d' "$TEMP_DIR/out")

    case "$outcome" in
      "Transaction is absent"*)
        echoerr
        echoerr "Transaction '$tx_hash' is absent"

        echoout "absent $tx_hash $domain"

        break
        ;;

      "Transaction is finalized"*)
        status=${outcome#'Transaction is finalized into block '*' with status "'}
        status=${status%'" and cost '*' CCD'*}

        case "$status" in
          success)
            echoout "success $tx_hash $domain"
            ;;

          rejected)
            # status examples:
            # 'register' in 'BictoryCns' at {"index":849,"subindex":0} failed with code -2147483635
            errcode=$(sed '2q;d' "$TEMP_DIR/out")
            errcode=${errcode#"'"*"' in '"*"' at "*" failed with code "}
            echoout "$errcode $tx_hash $domain"
            ;;

          *)
            echoerr "Error: Unable to parse status of finalized transaction"
            echoerr "Status: $outcome"
            echoout "sent $tx_hash $domain"
            ;;
        esac

        break
        ;;

      *)
        # At this point transaction is either accepted, pending or committed. Try to wait for finalization.
        fails=$((fails + 1))
        if [ "$fails" = 1 ]; then
          echoerr
          echoerr "Transaction '$tx_hash' is not finalized, current status: $outcome"
          echoerr "Waiting 5 seconds for finalization..."
        elif [ "$fails" -gt 12 ]; then
          echoerr "Error: minute without tx finalization, skipping transaction..."
          echoout "unfinalized $tx_hash $domain"
          break
        else
          echoerr "Current status: $outcome"
          echoerr "Waiting 5 more seconds..."
        fi

        # Wait for transaction to be finalized
        sleep 5
        ;;
    esac
  done

  unset DOMAIN_TX["$domain"]
done

# Reset variable to prevent overwrite on cleanup
unset DOMAIN_TX
