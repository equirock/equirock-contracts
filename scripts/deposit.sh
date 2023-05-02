./scripts/update_pyth.sh

CONTRACT_ADDR=$(<./scripts/contract-address)

SENDER=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck
USDT="peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5"
AMOUNT="1000000"

DEPOSIT_MSG=$(cat <<-END
    {
      "deposit": {
        "asset": {
          "info": {
            "native_token": {
              "denom": "%s"
            }
          },
          "amount": "%s"
        }
      }
    }
END
)

MSG=$(printf "$DEPOSIT_MSG" "$USDT" "$AMOUNT")

TX_HASH=$(injectived tx wasm execute "$CONTRACT_ADDR" "$MSG" --from $SENDER --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 --amount "$AMOUNT$USDT" -o json -y | jq '.txhash' -r)
sleep 5
echo $TX_HASH

CONTRACT_ADDR=$(injectived query tx "$TX_HASH" -o json | jq 'last(.logs[0].events[] | .attributes[] )' -r)
echo $CONTRACT_ADDR
