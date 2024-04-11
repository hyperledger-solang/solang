#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


function run_solc {
    if [ -z ${SOLC+x} ]; then
        SOLC=solc
    fi
    printf -- "    \033[1mRunning: \`\033[0;33msolc %s\033[0m\`..." "$*"
    if [ -z ${PRINT_COMPILER_OUTPUT+x} ]; then
        # Var is unset, so be quiet
        if solc "$@" >/dev/null 2>&1; then
            printf "\033[32mSUCCESS\n\033[0m"
            return 0
        else
            printf "\033[31mFAILED\n\033[0m"
            return 1
        fi
    else
        echo
        if solc "$@"; then
            printf "\033[32;1m    SUCCESS\n\033[0m\n"
            return 0
        else
            printf "\033[31;1m    FAILED\n\033[0m\n"
            return 1
        fi
        echo
    fi
}

function run_solang {

    if [ -z ${SOLANG+x} ]; then
        SOLANG=solang
    fi

    printf -- "    \033[1mRunning: \`\033[0;33m%s compile --target solana %s\033[0m\`..." "$SOLANG" "$*"

    if [ -z ${PRINT_COMPILER_OUTPUT+x} ]; then
        # Var is unset, so be quiet
        if "$SOLANG" compile --target solana -o .solang_out "$@" >/dev/null 2>&1; then
            printf "\033[32mSUCCESS\n\033[0m"
            return 0
        else
            printf "\033[31mFAILED\n\033[0m"
            return 1
        fi
    else
        echo
        if "$SOLANG" compile --target solana -o .solang_out "$@"; then
            printf "\033[32;1m    SUCCESS\n\033[0m\n"
            return 0
        else
            printf "\033[31;1m    FAILED\n\033[0m\n"
            return 1
        fi
    fi
}

function compare_runs {
    expected=$1
    solc_run=$2
    solang_run=$3
    name_1="$4"
    if [ -z ${4+x} ]; then
        name_1="solc"
    fi
    name_2="$5"
    if [ -z ${4+x} ]; then
        name_2="solang"
    fi

    if [ "$solc_run" -eq "$expected" ] && [ "$solc_run" -eq "$solang_run" ]; then

        printf "\033[1;32m" # GREEN BOLD
        printf "SUCCESS:"
        printf "\033[0m" # UNRED
        printf " %s and %s exit codes are expected value\n" "$name_1" "$name_2"
        return 0
    else
        printf "\033[1;31m" # RED BOLD
        echo "FAILURE:"
        printf "\033[0m" # UNRED
        echo "    expected: $expected"
        echo "    $name_1: $solc_run"
        echo "    $name_2: $solang_run"
        return 1
    fi
}

function print_test_banner {
    echo
    printf "\033[34;1m"
    printf "TEST %s:" "$1"
    printf "\033[0;1m"
    printf " %s\n" "$2"
    printf "\033[0m"
    echo
}
