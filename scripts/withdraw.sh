./scripts/update_pyth.sh

SENDER=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck
USDT="peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5"
CONTRACT_ADDR=$(<./scripts/contract-address)

LP_CONTRACT_ADDR=$(injectived query wasm contract-state smart "$CONTRACT_ADDR" '{"get_config": {}}' -o json | jq '.data.lp_token' -r)
LP_BALANCE=$(injectived query wasm contract-state smart "$LP_CONTRACT_ADDR" '{"balance": {"address": "'$SENDER'"}}' -o json | jq '.data.balance' -r)

SEND_MSG=$(cat <<-END
    {
      "withdraw": {}
    }
END
)
SEND_MSG_B64=$(echo $SEND_MSG | base64 -)


DEPOSIT_MSG=$(cat <<-END
    {
      "send": {
        "contract": "%s",
        "amount": "%s",
        "msg": "%s"
      }
    }
END
)

MSG=$(printf "$DEPOSIT_MSG" "$CONTRACT_ADDR" "$LP_BALANCE" "$SEND_MSG_B64")

TX_HASH=$(injectived tx wasm execute "$LP_CONTRACT_ADDR" "$MSG" --from $SENDER --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 -o json -y | jq '.txhash' -r)
sleep 5
echo $TX_HASH

EVENTS=$(injectived query tx "$TX_HASH" -o json | jq 'last(.logs[0].events[] )' -r)
echo $EVENTS
