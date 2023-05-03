
        contract Logic {
            struct LogicFields {
                uint logia;
            }
        }
        contract Math is Logic {
        }

        contract IsMath is Math {
            struct WithMath {
                LogicFields logia;
            }
        }
    
// ---- Expect: diagnostics ----
