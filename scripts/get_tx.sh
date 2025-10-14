#! /bin/bash

curl http://127.0.0.1:8547 \
  -X POST \
  -H "Content-Type: application/json" \
  --data "{\"method\":\"eth_getTransactionByHash\",\"params\":[\"$1\"],\"id\":1,\"jsonrpc\":\"2.0\"}" |
jq .
