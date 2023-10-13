@program_id("Ex9GgvN2ypqwFRGnsSZfAnXAnw5eRPDHyqRFnDWugxrb")
contract Test1 {
    function doThis() external returns (uint64) {
        return 3;
    }
}

contract Test2 {
    function callThat() public returns (uint64) {
        uint64 res = Test1.doThis();
        return res;
    }

    function callThat2() public returns (uint64) {
        // This is correct
        uint64 res = Test1.doThis{accounts: []}();
        return res;
    }
}

// ---- Expect: diagnostics ----
// error: 10:22-36: accounts are required for calling a contract. You can either provide the accounts with the {accounts: ...} call argument or change this function's visibility to external
