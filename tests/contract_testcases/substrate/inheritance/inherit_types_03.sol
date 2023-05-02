
        contract a {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }
        }

        contract b {
            enum enum_x { x1, x2 }
        }
        
// ---- Expect: diagnostics ----
// error: 3:45-51: type 'enum_x' not found
