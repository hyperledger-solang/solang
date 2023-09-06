#!/bin/bash
set -e

# Step 1: Compile the Solidity contract
solang compile -v --target polkadot flipper.sol

# Step 2: Upload the contract to a substrate node
upload_result=$(solang polkadot upload --suri //Alice -x flipper.contract --output-json)

# Step 3: Instantiate the contract
instantiate_result=$(solang polkadot instantiate --suri //Alice --args true -x flipper.contract --output-json --skip-confirm)

# Extract the contract address from the instantiate result
instantiate_contract_address=$(echo "$instantiate_result" | jq -r '.contract')

# Step 4: Call the contract
call_result=$(solang polkadot call --contract "$instantiate_contract_address" --message get --suri //Alice flipper.contract --output-json --skip-confirm)

# Extract the "value" field from the "data" object
value=$(echo "$call_result" | jq -r '.data.Bool')

# Step 5: Assert that "value" is true
if [ "$value" == "true" ]; then
    echo "Contract call succeeded."
else
    echo "Contract call reverted."
    exit 1
fi

# Step 6: Call the contract again to revert it
call_flip_result=$(solang polkadot call --contract "$instantiate_contract_address" --message flip --suri //Alice -x flipper.contract --output-json --skip-confirm)

# Step 7: Check that "value" is now false
call_result=$(solang polkadot call --contract "$instantiate_contract_address" --message get --suri //Alice flipper.contract --output-json --skip-confirm)

# Extract the "value" field from the "data" object
value=$(echo "$call_result" | jq -r '.data.Bool')

# Assert that "value" is false
if [ "$value" == "false" ]; then
    echo "Contract call reverted as expected."
else
    echo "Contract call did not revert as expected."
    exit 1
fi
