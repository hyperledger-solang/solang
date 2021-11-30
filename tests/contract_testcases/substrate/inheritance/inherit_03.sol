
        contract a {
            constructor(int arg1) public {
            }
        }

        contract b is a, a {
            constructor(int arg1) public {
            }
        }
        