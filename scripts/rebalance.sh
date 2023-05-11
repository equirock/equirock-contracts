
if [ $# -eq 0 ];then :;
else
    echo "Updating pyth"
    ./scripts/update_pyth.sh

    sleep 5
fi


CONTRACT_ADDR=$(<./scripts/contract-address)

SENDER=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck

DEPOSIT_MSG=$(cat <<-END
    {
      "rebalance": {
      }
    }
END
)

MSG=$(printf "$DEPOSIT_MSG")

TX_HASH=$(echo $KEYPASSWD | injectived tx wasm execute "$CONTRACT_ADDR" "$MSG" --from $SENDER --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 -o json -y | jq '.txhash' -r)
sleep 5
echo $TX_HASH

EVENTS=$(injectived query tx "$TX_HASH" -o json | jq 'last(.logs[0].events[] )' -r)
echo $EVENTS
