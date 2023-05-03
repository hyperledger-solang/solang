
        contract x {
            constructor() {}
        }

        contract c {
            using x for x;
        }
// ---- Expect: diagnostics ----
// error: 7:19-20: library expected but contract 'x' found
