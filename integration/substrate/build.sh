#!/bin/bash
set -e 

dup_contracts=$(grep -r '^contract .* {' | awk '{ print $2 }' | sort | uniq -d)
if [[ $dup_contracts ]]; then
	echo "Found contract with duplicate names: ${dup_contracts}"
	/bin/false
else
	parallel solang compile -v --target substrate --log-runtime-errors --math-overflow ::: *.sol test/*.sol;  solang compile debug_buffer_format.sol --target substrate -v --log-runtime-errors --log-api-return-codes
fi

