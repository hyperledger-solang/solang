#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"

test_failures=0

print_test_banner 1 "Multiple Remappings"
run_solc contracts/Contract.sol node_modules=resources/node_modules node_modules=node_modules --base-path resources
solc=$?
run_solang contracts/Contract.sol -m node_modules=resources/node_modules -m node_modules=node_modules -I resources
solang=$?
compare_runs 0 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 2 "Multiple Remappings 2"
run_solc contracts/Contract.sol node_modules=node_modules node_modules=resources/node_modules --base-path resources
solc=$?
run_solang contracts/Contract.sol -m node_modules=node_modules -m node_modules=resources/node_modules -I resources
solang=$?
compare_runs 1 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 3 "Multiple Remappings 3"
run_solc contracts/Contract.sol node_modules=node_modules node_modules=resources/node_modules node_modules=node_modules --base-path resources
solc=$?
run_solang contracts/Contract.sol -m node_modules=node_modules -m node_modules=resources/node_modules -m node_modules=node_modules -I resources
solang=$?
compare_runs 0 $solc $solang
test_failures=$((test_failures + $?))

exit $test_failures
