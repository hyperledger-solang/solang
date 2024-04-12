#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"

test_failures=0

print_test_banner 1 "No remapping"
run_solc contracts/Contract.sol
solc=$?
run_solang contracts/Contract.sol
solang=$?
compare_runs 1 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 2 "No base-path/import-path"
run_solc contracts/Contract.sol lib=node_modules/lib
solc=$?
run_solang contracts/Contract.sol -m lib=node_modules/lib
solang=$?
compare_runs 1 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 3 "Incomplete include-path/import-paths"
run_solc contracts/Contract.sol lib=node_modules/lib --base-path=.
solc=$?
run_solang contracts/Contract.sol -m "lib=node_modules/lib" -I .
solang=$?
compare_runs 1 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 4 "Incorrect include-path/import-paths"
run_solc contracts/Contract.sol lib=node_modules/lib --base-path=. --include-path=resources/node_modules
solc=$?
run_solang contracts/Contract.sol -m "lib=node_modules/lib" -I . -I resources/node_modules
solang=$?
compare_runs 1 $solc $solang
test_failures=$((test_failures + $?))

print_test_banner 5 "Correct configuration"
run_solc contracts/Contract.sol lib=node_modules/lib --base-path=. --include-path=resources
solc=$?
run_solang contracts/Contract.sol -m "lib=node_modules/lib" -I . -I resources
solang=$?
compare_runs 0 $solc $solang
test_failures=$((test_failures + $?))

exit $test_failures
