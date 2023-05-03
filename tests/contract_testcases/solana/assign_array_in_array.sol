// Ensure that subscript is assignable when member is array
contract C {
	function fixed() public {
		uint256[3][4] memory iPj;
		iPj[0] = [1,2,3];
	}

	function dynamic() public {
		uint256[][4] memory iPj;
		iPj[0] = new uint256[](4);
	}

	struct Sfixed { uint[3] f1; }
	struct Sdynamic { uint[] f1; }

	function fixed_struct() public {
		Sfixed iPj;
		iPj.f1 = [1,2,3];
	}

	function dynamic_struct() public {
		Sdynamic iPj;
		iPj.f1 = new uint256[](4);
	}
}

// ---- Expect: diagnostics ----
// warning: 3:2-25: function can be declared 'pure'
// warning: 4:24-27: local variable 'iPj' has been assigned, but never read
// warning: 8:2-27: function can be declared 'pure'
// warning: 9:23-26: local variable 'iPj' has been assigned, but never read
// warning: 16:2-32: function can be declared 'pure'
// warning: 17:10-13: local variable 'iPj' has been assigned, but never read
// warning: 21:2-34: function can be declared 'pure'
// warning: 22:12-15: local variable 'iPj' has been assigned, but never read
