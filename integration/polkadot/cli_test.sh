#!/bin/bash
set -e

# Compile the Solidity contract
solang compile -v --target polkadot flipper.sol

# Upload the contract to a substrate node
upload_result=$(solang polkadot upload --suri //Alice -x flipper.contract --output-json)

# Instantiate the contract
instantiate_result=$(solang polkadot instantiate --suri //Alice --args true -x flipper.contract --output-json --skip-confirm)

# Extract the contract address from the instantiate result
instantiate_contract_address=$(echo "$instantiate_result" | jq -r '.contract')

# Call the contract
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

# Call the flip function on the contract
call_flip_result=$(solang polkadot call --contract "$instantiate_contract_address" --message flip --suri //Alice -x flipper.contract --output-json --skip-confirm)

# Check that "value" is now false
call_result=$(solang polkadot call --contract "$instantiate_contract_address" --message get --suri //Alice flipper.contract --output-json --skip-confirm)

# Extract the "value" field from the "data" object
value=$(echo "$call_result" | jq -r '.data.Bool')

# Assert that "value" is false
if [ "$value" == "false" ]; then
    echo "Contract call flipped as expected."
else
    echo "Contract call did not flip the value."
    exit 1
fi

# Compile another Solidity contract
solang compile -v --target polkadot asserts.sol

# Upload the contract to a substrate node
upload_result=$(solang polkadot upload --suri //Alice -x asserts.contract --output-json)

# Extract the "code_hash" value directly from the second JSON object
code_hash=$(echo "$upload_result" | tail -n 1 | jq -r '.code_hash')

# Remove the contract using the code hash
remove_result=$(solang polkadot remove --suri //Alice --output-json --code-hash "$code_hash" asserts.contract)

# Get the "code_hash" value from the second JSON object
removed_code_hash=$(echo "$remove_result" | tail -n 1)

# In case of success, the last line of the output will be the code hash
if [[ "$removed_code_hash" == 0x* ]]; then
    echo "Contract removed successfully."
    exit 0
else
    echo "Contract removal failed."
    exit 1
fi
