
        abstract contract test {
            constructor() internal {}
        }
// ---- Expect: diagnostics ----
// warning: 3:27-35: 'internal': visibility for constructors is ignored
