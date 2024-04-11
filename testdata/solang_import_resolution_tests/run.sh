#!/usr/bin/env bash
# SPDX-License-Identifier: Apache-2.0


function print_test_set_banner {

    echo
    printf "\033[36;1;4m////////////////////////////////////////////////////////////////////////////////\033[0m\n"
    printf "\033[36;1;4m////////////////////////\033[0;1m       RUNNING TEST SET %s       \033[36;1;4m////////////////////////\033[0m\n" "$1"
    printf "\033[36;1;4m////////////////////////////////////////////////////////////////////////////////\033[0m\n"
    echo
}

function color_exit_code {
    if [ "$1" -eq "0" ]; then
        printf "\033[32;1m%s\033[0m" "$1"
    else
        printf "\033[31;1m%s\033[0m" "$1"
    fi
}

function run_tests {

    print_test_set_banner 1
    cd 01_solang_remap_target || exit 1
    ./run.sh
    failures_1=$?
    cd .. || exit 1

    print_test_set_banner 2
    cd 02_solang_incorrect_direct_imports || exit 1
    ./run.sh
    failures_2=$?
    cd .. || exit

    print_test_set_banner 3
    cd 03_ambiguous_imports_should_fail || exit 1
    ./run.sh
    failures_3=$?
    cd .. || exit

    print_test_set_banner 4
    cd 04_multiple_map_path_segments || exit 1
    ./run.sh
    failures_4=$?
    cd .. || exit

    print_test_set_banner 5
    cd 05_import_path_order_should_not_matter || exit 1
    ./run.sh
    failures_5=$?
    cd .. || exit

    print_test_set_banner 6
    cd 06_redundant_remaps || exit 1
    ./run.sh
    failures_6=$?
    cd .. || exit
}

if [ -z ${QUIET+x} ]; then
    run_tests
else
    run_tests >/dev/null
fi

total_failures=$((failures_1 + failures_2 + failures_3 + failures_4 + failures_5 + failures_6))

printf "\n                 \033[1mSummary\033[0m\n\n"
printf "\033[1m01_solang_remap_target\033[0m:                    %s\n" "$(color_exit_code $failures_1)"
printf "\033[1m02_solang_incorrect_direct_imports\033[0m:        %s\n" "$(color_exit_code $failures_2)"
printf "\033[1m03_solang_permissive_on_ambiguous_imports\033[0m: %s\n" "$(color_exit_code $failures_3)"
printf "\033[1m04_multiple_map_path_segments\033[0m:             %s\n" "$(color_exit_code $failures_4)"
printf "\033[1m05_import_path_order_should_not_matter\033[0m:    %s\n" "$(color_exit_code $failures_5)"
printf "\033[1m06_redundant_remaps\033[0m:                       %s\n" "$(color_exit_code $failures_6)"
printf -- "---------------------------------------------\n"
printf "\033[1mTotal Test Failures\033[0m:                       %s\n" "$(color_exit_code $total_failures)"
