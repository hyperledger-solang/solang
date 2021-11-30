
        contract base {
            constructor(bool x) {}
        }

        contract apex is base {
                function foo() pure public {}
        }