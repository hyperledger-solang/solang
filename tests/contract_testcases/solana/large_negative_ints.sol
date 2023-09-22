pragma solidity ^0.8.0;

library Issue1523 {
    function i1523_fail1(int256 x) internal pure returns (bool) {
        return x <= type(int256).min;
    }

    function i1523_fail2(int256 x) internal pure returns (bool) {
        return x <= type(int256).min + 1;
    }

    function i1523_fail3(int256 x) internal pure returns (bool) {
        // Actual min value inlined
        return x <= -57896044618658097711785492504343953926634992332820282019728792003956564819968;
    }

    function i1523_fail4(int256 x) internal pure returns (bool) {
        // Actual min value + 1
        return x <= -57896044618658097711785492504343953926634992332820282019728792003956564819967;
    }

    function i1523_pass1() internal pure returns (int256) {
        return type(int256).min + 1;
    }

    function i1523_pass2() internal pure returns (int256) {
        return type(int256).min;
    }
}

// ---- Expect: diagnostics ----
