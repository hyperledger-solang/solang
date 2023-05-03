
        contract c {
            mapping(uint => bool) data;
            function test() public {
                delete data;
            }
        }
// ---- Expect: diagnostics ----
// error: 5:17-28: 'delete' cannot be applied to mapping type
