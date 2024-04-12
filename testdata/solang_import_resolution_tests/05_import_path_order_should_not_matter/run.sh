#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


# shellcheck source=/dev/null
source "../util.sh"

test_failures=0

print_test_banner 1 "Testing commutativity of import path"
if [ -e .solang_out1 ]; then
    rm -rf .solang_out1
fi
if [ -e .solang_out2 ]; then
    rm -rf .solang_out2
fi

run_solang contracts/Contract.sol -I contracts/nested1 -I contracts/nested2
solang_run_1=$?
[ -e .solang_out ] && mv .solang_out .solang_out1

run_solang contracts/Contract.sol -I contracts/nested2 -I contracts/nested1
solang_run_2=$?
[ -e .solang_out ] && mv .solang_out .solang_out2

compare_runs 1 $solang_run_1 $solang_run_2 "solang_1" "solang_2"
test_failures=$((test_failures + $?))

if [ -e .solang_out1 ] && [ -e .solang_out2 ]; then
    if diff -r .solang_out1 .solang_out2; then
        printf "\033[31;1mFAILURE:\033[0m different runs produced different outputs: %s %s\n" .solang_out1 .solang_out2
    else
        printf "\033[32;1mSUCCESS:\033[0m different runs produced same outputs: \033[1m%s %s\033[0m\n" .solang_out1 .solang_out2
    fi
elif [ -e .solang_out1 ] || [ -e .solang_out2 ]; then
    printf "\033[31;1mFAILURE:\033[0m could not compare Solang outputs: missing at least one of: \033[1m%s %s\033[0m\n" .solang_out1 .solang_out2
    test_failures=$((test_failures + 1))
fi

exit $test_failures
