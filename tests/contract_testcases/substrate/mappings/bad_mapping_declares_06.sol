
        contract c {
            function test() public returns (mapping(int => address) x) {
                //
            }
        }
// ----
// error (66-89): return type containing mapping must be of type 'storage'
