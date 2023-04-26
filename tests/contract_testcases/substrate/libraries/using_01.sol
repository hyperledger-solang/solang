
        contract x {
            constructor() {}
        }

        contract c {
            using x for x;
        }
// ----
// error (101-102): library expected but contract 'x' found
