TX_HASH=$(injectived tx wasm store artifacts/equirock_contracts.wasm --from howlpack --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 -o json -y | jq '.txhash' -r)
sleep 3
CODE_ID=$(injectived query tx "$TX_HASH" -o json | jq '.logs[0].events[] | select(.type=="store_code") | .attributes[] | select(.key=="code_id") | .value' -r)
echo $CODE_ID

ADMIN=inj1e79v7eyl9yxjnmperuzzfd3w7q495na9hq4xck
CW20_CODE_ID=699
ETF_TOKEN_NAME="ER-STRATEGY-1"
USDT="peggy0xdAC17F958D2ee523a2206206994597C13D831ec7"
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
          "assets": []
        }
    }
END
)

MSG=$(printf "$INSTANTIATE_MSG" "$CW20_CODE_ID" "$ETF_TOKEN_NAME" "$USDT")

TX_HASH=$(injectived tx wasm instantiate $CODE_ID "$MSG" --from $ADMIN --admin $ADMIN --gas-prices 500000000inj --gas auto --gas-adjustment 1.3 --label "$ETF_TOKEN_NAME" -o json -y | jq '.txhash' -r)
sleep 3
echo $TX_HASH

CONTRACT_ADDR=$(injectived query tx "$TX_HASH" -o json | jq 'last(.logs[0].events[] | .attributes[] | select(.key=="contract_address") | .value)' -r)
echo $CONTRACT_ADDR
