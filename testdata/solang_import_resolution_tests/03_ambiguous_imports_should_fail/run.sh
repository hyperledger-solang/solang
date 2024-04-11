#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"
test_failures=0

print_test_banner 1 "Ambiguous Imports Should Fail"
run_solc contracts/Contract.sol lib=resources/node_modules/lib --base-path . --include-path contracts
solc_run=$?
run_solang contracts/Contract.sol -m "lib=resources/node_modules/lib" -I contracts -I .
solang_run=$?
compare_runs 1 $solc_run $solang_run
test_failures=$((test_failures + $?))

print_test_banner 2 "Import Order Shouldn't Matter"
run_solang contracts/Contract.sol -m "lib=resources/node_modules/lib" -I resources/node_modules/lib -I .
solang_run_1=$?
run_solang contracts/Contract.sol -m "lib=resources/node_modules/lib" -I . -I resources/node_modules/lib
solang_run_2=$?

if [ $solang_run_1 -eq $solang_run_2 ]; then

    printf "\033[1;32m" # GREEN BOLD
    printf "SUCCESS:"
    printf "\033[0m" # UNGREEN
    printf " both solang runs evaluated the same \n"
else
    printf "\033[1;31m" # RED BOLD
    echo "FAILURE:"
    printf "\033[0m" # UNRED
    echo "    solang run 1:     $solang_run_1"
    echo "    solang run 2:     $solang_run_2"
    test_failures=$((test_failures + 1))
fi

exit $test_failures
