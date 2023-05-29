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
// error: 13:24-31: functions declared external cannot be called via an internal function call
// 	note 6:5-47: declaration of function 'total'
