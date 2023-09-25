#!/bin/bash
set -e

# Compile the Solidity contract to a Solana program
solang compile --target solana flipper.sol

# Deploy the program (assumes a localnet cluster is running)
deploy_result=$(solang solana deploy --output-json flipper.so)

# Extract the program id from the deploy result
program_id=$(echo $deploy_result | jq -r .programId)

# Assert the program is deployed successfully
# (the program id is not null)
if [ "$program_id" == "null" ]; then
  echo "Error: Solana program deployment failed"
  exit 1
fi

exit 0
