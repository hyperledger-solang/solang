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
