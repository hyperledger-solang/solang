
        contract c {
            function test() public returns (mapping(int => address) storage x) {
                //
            }
        }
// ---- Expect: diagnostics ----
// error: 3:69-76: return type of type 'storage' not allowed public or external functions
