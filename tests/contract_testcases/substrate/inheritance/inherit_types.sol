
        contract a is b {
            function test() public returns (enum_x) {
                return enum_x.x2;
            }
        }

        abstract contract b {
            enum enum_x { x1, x2 }
        }
        