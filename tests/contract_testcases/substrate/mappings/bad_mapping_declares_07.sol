
        contract c {
            function test() public returns (mapping(int => address) storage x) {
                //
            }
        }
// ----
// error (90-97): return type of type 'storage' not allowed public or external functions
