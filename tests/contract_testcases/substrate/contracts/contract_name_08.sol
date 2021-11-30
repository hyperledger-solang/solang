
        contract a {
            function x() public {
                b y = new b();
            }
        }

        contract b {
            function x() public {
                c y = new c();
            }
        }

        contract c {
            function x() public {
                a y = new a();
            }
        }
        