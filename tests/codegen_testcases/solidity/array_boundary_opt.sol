// RUN: --target polkadot --emit cfg

contract Array_bound_Test {
    // BEGIN-CHECK: Array_bound_Test::Array_bound_Test::function::array_bound__uint256:
    function array_bound(
        uint256[] b,
        uint256 size,
        uint32 size32
    ) public pure returns (uint256) {
        // CHECK: ty:uint32 %1.cse_temp = (trunc uint32 (arg #1))
	    // CHECK: ty:uint32 %array_length.temp.32 = %1.cse_temp
        uint256[] a = new uint256[](size);

        // CHECK: ty:uint32 %array_length.temp.33 = (arg #2)
        uint256[] c = new uint256[](size32);

        // CHECK: ty:uint32 %array_length.temp.34 = uint32 20
        uint256[] d = new uint256[](20);

        // CHECK: ty:uint32 %array_length.temp.32 = (overflowing %1.cse_temp + uint32 1)
        a.push();

        // CHECK: ty:uint32 %array_length.temp.33 = (overflowing (arg #2) - uint32 1)
        c.pop();

        // CHECK: ty:uint32 %array_length.temp.34 = uint32 21
        d.push();

        // CHECK: return (zext uint256 (((%array_length.temp.32 + (builtin ArrayLength ((arg #0)))) + (overflowing (arg #2) - uint32 1)) + uint32 21))
        return a.length + b.length + c.length + d.length;
    }

    // BEGIN-CHECK: function Array_bound_Test::Array_bound_Test::function::test__bool
    function test(bool cond) public returns (uint32) {
        bool[] b = new bool[](210);

        if (cond) {
            // CHECK: ty:uint32 %array_length.temp.39 = uint32 211
            b.push(true);
        }

        // CHECK: return %array_length.temp.39
        return b.length;
    }

    // BEGIN_CHECK: function Array_bound_Test::Array_bound_Test::function::test_for_loop
    function test_for_loop() public {
        uint256[] a = new uint256[](20);
        a.push(1);
        uint256 sesa = 0;


        // CHECK: branchcond (unsigned uint32 20 >= uint32 21), block5, block6
        // CHECK: branchcond (unsigned less %i < uint256 21), block1, block4
        for (uint256 i = 0; i < a.length; i++) {
            sesa = sesa + a[20];
        }

        assert(sesa == 21);
    }

    // BEGIN_CHECK: function Array_bound_Test::Array_bound_Test::function::test_without_size
    function test_without_size(uint[] dyn_param) public returns (uint32) {
        uint256[] vec;
        vec.push(3);
        uint256[] vec2;
        vec2=vec;
        uint256[] a = new uint256[](20);
        uint256[] b = dyn_param;
        uint256[] c = vec;


        // CHECK: return ((uint32 21 + (builtin ArrayLength ((arg #0)))) + uint32 1)
        return vec2.length + a.length + b.length + c.length;
    }

    // BEGIN_CHECK: function Array_bound_Test::Array_bound_Test::function::test_loop_2
    function test_loop_2() public {
        int256[] vec = new int256[](10);

        for (int256 i = 0; i < 5; i++) {
            // CHECK: branchcond (unsigned more %array_length.temp.49 > uint32 20), block5, block6
            if (vec.length > 20) {
                break;
            }
            vec.push(3);
        }

        // CHECK: branchcond (%array_length.temp.49 == uint32 15), block7, block8
        assert(vec.length == 15);
    }

    // BEGIN-CHECK: Array_bound_Test::Array_bound_Test::function::getVec__int32_int32
    function getVec(int32 a, int32 b) public pure returns (uint32) {
        int32[] memory vec;
        vec = [a, b];
        // CHECK: ty:int32[] %vec = undef
	    // CHECK: ty:uint32 %array_length.temp.52 = uint32 0
	    // CHECK: ty:int32[] %temp.53 = (alloc int32[] len uint32 2)
        // CHECK: ty:uint32 %array_length.temp.54 = uint32 2
	    // CHECK: ty:int32[] %vec = %temp.53


        vec.push(5);
        // CHECK: ty:uint32 %array_length.temp.54 = uint32 3
        // CHECK: return uint32 3
        return vec.length;
    }

    // BEGIN-CHECK: Array_bound_Test::Array_bound_Test::function::testVec__uint32_uint32_uint32
    function testVec(uint32 a, uint32 b, uint32 c) public pure returns (uint32) {
        // CHECK: ty:uint32[] %temp.56 = (alloc uint32[] len uint32 3)
        // CHECK: ty:uint32 %array_length.temp.57 = uint32 3
        uint32[] memory vec = [a, b, b];
        // CHECK: ty:uint32[] %vec = %temp.56

        vec.pop();
        // CHECK: ty:uint32 %array_length.temp.57 = uint32 2
        // CHECK: return uint32 2
        return vec.length;
    }


}
