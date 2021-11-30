
        contract x {
            constructor() {}
        }

        contract c {
            using x for x;
        }