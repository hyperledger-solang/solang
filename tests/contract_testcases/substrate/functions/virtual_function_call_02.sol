interface I {
    function total() external returns (int256);
}

contract A {
    function total() external returns (int256) {
        return 4;
    }
}

contract B is I, A {
    function x() public {
        int256 total = total();
    }
}

// ---- Expect: diagnostics ----
// error: 13:24-29: unknown function or type 'total'\n"`: source: tests/contract_testcases/substrate/functions/virtual_function_call_02.sol
