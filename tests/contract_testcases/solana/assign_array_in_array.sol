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

// ----
// warning (74-97): function can be declared 'pure'
// warning (123-126): local variable 'iPj' has been assigned, but never read
// warning (153-178): function can be declared 'pure'
// warning (203-206): local variable 'iPj' has been assigned, but never read
// warning (306-336): function can be declared 'pure'
// warning (348-351): local variable 'iPj' has been assigned, but never read
// warning (378-410): function can be declared 'pure'
// warning (424-427): local variable 'iPj' has been assigned, but never read
