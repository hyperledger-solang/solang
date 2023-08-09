contract testing {
    function general_test(uint64 a) public view returns (uint64, uint256) {
        uint64 g = 0;
        uint256 h = 0;
        assembly {
            function sum(a, b) -> ret1 {
                ret1 := add(a, b)
            }

            function mix(a, b) -> ret1, ret2 {
                ret1 := mul(a, b)
                ret2 := add(a, b)
            }

            for {
                let i := 0
            } lt(i, 10) {
                i := add(i, 1)
            } {
                if eq(a, 259) {
                    break
                }
                g := sum(g, 2)
                if gt(a, 10) {
                    continue
                }
                g := sub(g, 1)
            }

            if or(lt(a, 10), eq(a, 259)) {
                g, h := mix(g, 10)
            }
        }

        return (g, h);
    }
}
