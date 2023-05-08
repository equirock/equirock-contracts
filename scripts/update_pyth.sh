SENDER=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck

INJ_PRICEFEED_ID="2d9315a88f3019f8efa88dfe9c0f0843712da0bac814461e27733f6b83eb51b3"
ATOM_PRICEFEED_ID="61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3"

PRICEED_IDS="ids[]=$INJ_PRICEFEED_ID&ids[]=$ATOM_PRICEFEED_ID"
PYTH_CONTRACT_ADDR=inj1z60tg0tekdzcasenhuuwq3htjcd5slmgf7gpez

data=$(curl -s https://xc-testnet.pyth.network/api/latest_vaas?$PRICEED_IDS | jq '.' -r)

UPDATE_VAA=$(cat <<-END
  {
    "update_price_feeds": {
      "data": %s
    }
  }
END
)

MSG=$(printf "$UPDATE_VAA" "$data" )
TX_HASH=$(echo $KEYPASSWD | injectived tx wasm execute "$PYTH_CONTRACT_ADDR" "$MSG" --from $SENDER --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 --amount 1000000000000000000inj -o json -y | jq '.txhash' -r)

echo $TX_HASH

