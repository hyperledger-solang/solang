
        contract c {
            function test(mapping(int => address) x) public {
                //
            }
        }
// ----
// error (48-71): parameter with mapping type must be of type 'storage'
