
        contract a is b {
            foo public var1;
        }

        abstract contract b {
            struct foo {
                uint32 f1;
                uint32 f2;
            }
        }
        
// ---- Expect: diagnostics ----
