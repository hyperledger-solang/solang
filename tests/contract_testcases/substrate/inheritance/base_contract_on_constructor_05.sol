
        abstract contract base {
            constructor(bool x) {}
        }

        contract apex is base {
            constructor() base(true) base(false) {}
            function foo() pure public {}
        }
// ---- Expect: diagnostics ----
// warning: 3:30-31: function parameter 'x' has never been read
// error: 7:38-49: duplicate base contract 'base'
// 	note 7:27-37: previous base contract 'base'
