contract Array_bound_Test {
    function array_bound() public pure returns (uint256) {
        uint256[] a = new uint256[](10);
        uint256 sum = 0;

        if (2>1) {
	    a.push(5);
	} else {
	    a.pop();
	}

        for (uint256 i = 0; i < a.length; i++) {
	    sum = sum + a[10];
	}

        return sum;
    }
}

// ---- Expect: diagnostics ----
