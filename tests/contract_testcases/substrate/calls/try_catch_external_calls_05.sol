
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 bla, bool) {
                    x = bla;
                } catch Error(bytes memory f) {
                    x = 105;
                } catch Panic(uint128 code) {
                    x = 106;
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
