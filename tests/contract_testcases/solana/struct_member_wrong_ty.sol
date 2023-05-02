contract DeleteTest {

        struct tt {
            int[4] vec;
        }

    function getVec() public returns (int) {
        tt memory testing = tt({vec: [int(1), 2, 3, 4]});
        int[] memory ret = testing.vec;
        return ret[2];
    }

}

// ---- Expect: diagnostics ----
// error: 9:36-39: conversion from int256[4] to int256[] not possible
