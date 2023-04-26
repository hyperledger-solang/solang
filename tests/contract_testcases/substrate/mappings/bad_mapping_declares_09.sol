
        contract c {
            mapping(uint => bool) data;
            function test() public {
                delete data;
            }
        }
// ----
// error (115-126): 'delete' cannot be applied to mapping type
