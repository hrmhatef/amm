# Simple AMM on near platform

#### Build
```
rustup target add wasm32-unknown-unknown

./build.sh
```
#### Tokens
You can use the following files to deploy **Token A and B** on near network
 - ./deploy_ft_a.sh
 - ./deploy_ft_b.sh

Next you should setup your tokens:
```bash
export ID = <your root account ID>

# Create contract with default meta
near call token_a.$ID new_default_meta '{"owner_id": "token_a.<ID>","total_supply": "1000000"}' --accountId $ID;

# Set storage deposit
near call token_a.$ID storage_deposit '{"account_id": "rick.<ID>"}' --accountId $ID --deposit 1 --gas 25000000000000;
near call token_a.$ID storage_deposit '{"account_id": "amm.<ID>"}' --accountId $ID --deposit 1 --gas 25000000000000;

# Send tokens to Rick
near call token_a.$ID ft_transfer '{"receiver_id": "rick.<ID>", "amount": "500000"}' --accountId token_a.$ID --depositYocto 1;

# For contract B, do the same
```

#### You can use deploy_amm.sh to deploy **AMM Contract**
#### Setup AMM contract
```bash
# Init contract
near call amm.$ID new '{
    "owner_id": "amm.$ID",
    "token_a_id": "token_a.<ID>",
    "token_b_id": "token_b.<ID>",
    }' --accountId amm.$ID;
```
#### You need to set Metadata of tokens with the following command:
```bash
near call amm.$ID set_metadata_a '{
"metadata": { "spec": "ft-1.0.0", "name": "Example Token Name", "symbol": "FTA", "decimals": 6 }
}' --accountId amm.$ID
# For contract B, do the same with method **set_metadata_b**
```

# Set storage deposit to Rick
```bash
near call amm.$ID storage_deposit '{"token_name":"token_a.<ID>","account_id": "rick.<ID>"}' --accountId amm.$ID --deposit 1 --gas 25000000000000;
```


For send tokens from FT to AMM use FT.ft_transfer_call

For add token to pool use AMM.add_token_to_pool

For exclude token from pool use AMM.exclude_token_from_pool

For swap tokens use AMM.swap

For withdraw tokens use AMM.withdraw_tokens

For get metadata of token a use AMM.ft_metadata_a

For get metadata of token a use AMM.ft_metadata_b

for get contract info use AMM.contract_info


## Test
```bash
cargo test --all
```
