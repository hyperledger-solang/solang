
        contract c {
            function test() public {
                try new int32[](2) returns (int32, bool) {
                    x = 1;
                } catch (string) {
                    x = 2;
                }
                assert(x == 1);
            }
        }

        contract other {
            function test() public returns (int32, bool) {
                return (102, true);
            }
        }
        