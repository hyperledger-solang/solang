
        contract c {
            function test(mapping(int => address) storage x) public {
                //
            }
        }
// ----
// error (72-79): parameter of type 'storage' not allowed public or external functions
