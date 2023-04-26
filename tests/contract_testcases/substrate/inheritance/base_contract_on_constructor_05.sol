
        abstract contract base {
            constructor(bool x) {}
        }

        contract apex is base {
            constructor() base(true) base(false) {}
            function foo() pure public {}
        }
// ----
// warning (63-64): function parameter 'x' has never been read
// error (149-160): duplicate base contract 'base'
// 	note (138-148): previous base contract 'base'
