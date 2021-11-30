
        contract base {
            struct s { uint32 f1; }
        }

        contract b {
            struct s { uint32 f1; }
        }

        contract apex is base {
            constructor() public b {

            }
        }