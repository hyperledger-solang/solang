#!/bin/bash
set -ex
CACHE=/Users/hyp/cache
FILE=$CACHE/$1
if [ ! -d $CACHE ]; then
    # not running on Hyperledger CI, do not cache
    curl -L --output $1 https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/$1
elif [ -f "$1" ]; then
    cp $1 $FILE
elif [ -f "$FILE" ]; then
    cp $FILE .
else
    curl -L --output $FILE https://github.com/hyperledger-labs/solang/releases/download/v0.1.11/$1
    cp $FILE .
fi
