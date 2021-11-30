
        contract a is b {
            constructor(int arg1) public {
            }
        }

        contract b is c {
            constructor(int arg1) public {
            }
        }

        contract c is a {
            constructor(int arg1) public {
            }
        }
        