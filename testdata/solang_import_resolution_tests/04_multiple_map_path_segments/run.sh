#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"

test_failures=0

print_test_banner 1 "Multiple import mapping segments should be supported"
run_solc contracts/Contract.sol lib/nested=resources/node_modules/lib/nested --base-path .
solc_run=$?
run_solang contracts/Contract.sol -m "lib/nested=resources/node_modules/lib/nested" -I .
solang_run=$?
compare_runs 0 $solc_run $solang_run
test_failures=$((test_failures + $?))

exit $test_failures
