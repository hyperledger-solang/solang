contract c {
    int[2] case1;

    function f(int[2] storage case2) internal {
        case1[0] = 1;
        case2[0] = 1;
	g()[1] = 2;
    }

    function g() internal view returns (int[2] storage) {
	    return case1;
    }

    function test() public {
	    f(case1);
    }
}

// ---- Expect: diagnostics ----
