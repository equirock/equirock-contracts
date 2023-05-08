# Equirock

## CLI Usage

1. `echo "inj1603kayackkckj50dqsdaka8kz875xksxwas23r" | tee ./scripts/contract-address`
2. `KEYPASSWD=your-injective-keyring-pass ./scripts/deposit.sh true`
3. `KEYPASSWD=your-injective-keyring-pass ./scripts/withdraw.sh true`

On the Injective testnet Helix, the availability of assets on the spot market is quite limited. As a result, only Injective can be found among the underlying basket assets. However, there is a further complication as the spot market price on the testnet often differs significantly from the actual price. This can result in high slippage of up to 10% when attempting to match testnet order book transactions.

```
Testnet

INJ/USDT marketid 0x0611780ba69656949525013d947713300f56c37b6175e02f26bffa495c3208fe

```
