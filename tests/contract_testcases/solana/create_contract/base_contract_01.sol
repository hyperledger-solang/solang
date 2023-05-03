
        contract Logic {
            enum LogicError {
                LE_ERROR
            }
        }
        contract Math is Logic {
            enum MathError {
                NO_ERROR
            }
        }

        contract IsMath is Math {
            struct WithMath {
                MathError math;
                LogicError logic;
            }
        }
    
// ---- Expect: diagnostics ----
