#!/bin/bash
set -e

dup_contracts=$(grep -r '^contract .* {' | awk '{ print $2 }' | sort | uniq -d)
if [[ $dup_contracts ]]; then
	echo "Found contract with duplicate names: ${dup_contracts}"
	/bin/false
else
	parallel solang compile -v -g --target substrate ::: *.sol test/*.sol ; solang compile -v --target substrate --release release_version.sol ;
fi

