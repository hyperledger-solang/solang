
        abstract contract base {
            constructor(bool x) {}
        }

        contract apex is base {
            constructor() base(true) base(false) {}
            function foo() pure public {}
        }