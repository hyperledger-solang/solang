
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                a y = new a();
            }
        }
        