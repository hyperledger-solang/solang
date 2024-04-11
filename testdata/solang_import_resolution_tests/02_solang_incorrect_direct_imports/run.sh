#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"

test_failures=0

print_test_banner 1 "Correct Configs (should pass)"
run_solc contracts/Contract.sol
solc_run=$?
run_solang contracts/Contract.sol
solang_run=$?
compare_runs 0 $solc_run $solang_run
test_failures=$((test_failures + $?))

print_test_banner 2 "Correct Configs (should pass)"
run_solc contracts/Contract.sol --base-path .
solc_run=$?
run_solang contracts/Contract.sol -I .
solang_run=$?
compare_runs 0 $solc_run $solang_run
test_failures=$((test_failures + $?))

exit $test_failures
