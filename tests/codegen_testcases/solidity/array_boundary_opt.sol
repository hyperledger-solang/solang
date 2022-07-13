// RUN: --target substrate --emit cfg

contract Array_bound_Test {
    // BEGIN-CHECK: Array_bound_Test::Array_bound_Test::function::array_bound__uint256:
    function array_bound(
        uint256[] b,
        uint256 size,
        uint32 size32
    ) public pure returns (uint256) {
        // CHECK: ty:uint32 %array_length.temp.23 = (trunc uint32 (arg #1))
        uint256[] a = new uint256[](size);

        // CHECK: ty:uint32 %array_length.temp.24 = (arg #2)
        uint256[] c = new uint256[](size32);

        // CHECK: ty:uint32 %array_length.temp.25 = uint32 20
        uint256[] d = new uint256[](20);

        // CHECK: ty:uint32 %array_length.temp.23 = (%array_length.temp.23 + uint32 1)
        a.push();

        // CHECK: ty:uint32 %array_length.temp.24 = ((arg #2) - uint32 1)
        c.pop();

        // CHECK: ty:uint32 %array_length.temp.25 = uint32 21
        d.push();

        // CHECK: return (zext uint256 (((%array_length.temp.23 + (builtin ArrayLength ((arg #0)))) + ((arg #2) - uint32 1)) + uint32 21))
        return a.length + b.length + c.length + d.length;
    }

    // BEGIN-CHECK: function Array_bound_Test::Array_bound_Test::function::test__bool
    function test(bool cond) public returns (uint32) {
        bool[] b = new bool[](210);

        if (cond) {
            // CHECK: ty:uint32 %array_length.temp.29 = uint32 211
            b.push(true);
        }

        // CHECK: return %array_length.temp.29
        return b.length;
    }

    // BEGIN_CHECK: function Array_bound_Test::Array_bound_Test::function::test_for_loop
    function test_for_loop() public {
        uint256[] a = new uint256[](20);
        a.push(1);
        uint256 sesa = 0;


        // CHECK: branchcond (uint32 20 >= uint32 21), block5, block6
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
            // CHECK: branchcond (unsigned more %array_length.temp.39 > uint32 20), block5, block6
            if (vec.length > 20) {
                break;
            }
            vec.push(3);
        }

        // CHECK: branchcond (%array_length.temp.39 == uint32 15), block7, block8
        assert(vec.length == 15);
    }


}
