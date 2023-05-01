docker run --rm -v "$(pwd)":/code --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry cosmwasm/rust-optimizer:0.12.11

TX_HASH=$(injectived tx wasm store artifacts/equirock_contracts.wasm --from howlpack --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 -o json -y | jq '.txhash' -r)
sleep 5
CODE_ID=$(injectived query tx "$TX_HASH" -o json | jq '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value' -r)
echo $CODE_ID

ADMIN=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck
CW20_CODE_ID=699
ETF_TOKEN_NAME="ER-STRATEGY-1"
USDT="peggy0x87aB3B4C8661e07D6372361211B96ed4Dc36B1B5"
INSTANTIATE_MSG=$(cat <<-END
    {
        "etf_token_code_id": %s,
        "etf_token_name": "%s",
        "deposit_asset": {
          "native_token": {
            "denom": "%s"
          }
        },
        "pyth_contract_addr": "inj1z60tg0tekdzcasenhuuwq3htjcd5slmgf7gpez",
        "basket": {
          "assets": [{
            "asset": {
              "info": {
                "native_token": {
                  "denom": "inj"
                }
              },
              "amount": "0"
            },
            "weight": "3",
            "pyth_price_feed": "2d9315a88f3019f8efa88dfe9c0f0843712da0bac814461e27733f6b83eb51b3",
            "spot_market_id": "0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe"
          },{
            "asset": {
              "info": {
                "native_token": {
                  "denom": "factory/inj17vytdwqczqz72j65saukplrktd4gyfme5agf6c/atom"
                }
              },
              "amount": "0"
            },
            "weight": "1",
            "pyth_price_feed": "61226d39beea19d334f17c2febce27e12646d84675924ebb02b9cdaea68727e3",
            "spot_market_id": "0x491ee4fae7956dd72b6a97805046ffef65892e1d3254c559c18056a519b2ca15"
          }]
        }
    }
END
)



MSG=$(printf "$INSTANTIATE_MSG" "$CW20_CODE_ID" "$ETF_TOKEN_NAME" "$USDT")

TX_HASH=$(injectived tx wasm instantiate $CODE_ID "$MSG" --from $ADMIN --admin $ADMIN --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 --label "$ETF_TOKEN_NAME" -o json -y | jq '.txhash' -r)
sleep 5
echo $TX_HASH

CONTRACT_ADDR=$(injectived query tx "$TX_HASH" -o json | jq 'last(.logs[0].events[] | .attributes[] | select(.key=="contract_address") | .value)' -r)
CONTRACT_ADDR=${CONTRACT_ADDR:1:-1}
echo $CONTRACT_ADDR | tee ./scripts/contract-address

