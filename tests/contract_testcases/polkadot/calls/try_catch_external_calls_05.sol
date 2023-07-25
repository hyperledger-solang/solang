
        contract c {
            function test() public {
                other o = new other();
                int32 x = 0;
                try o.test() returns (int32 bla, bool) {
                    x = bla;
                } catch Error(bytes memory f) {
                    x = 105;
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

// ---- Expect: diagnostics ----
// error: 8:31-36: catch Error(...) can only take 'string memory', not 'bytes'
// error: 10:26-32: catch can only take 'bytes memory', not 'string'
