
contract default {
	function switch(bool case) public pure returns (bool) { 
		return !case;
	}
}

// ---- Expect: diagnostics ----
