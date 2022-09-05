
        abstract contract base {
            struct s { uint32 f1; }
        }

        abstract contract b {
            struct s { uint32 f1; }
        }

        abstract contract apex is base {
            constructor() public b {

            }
        }