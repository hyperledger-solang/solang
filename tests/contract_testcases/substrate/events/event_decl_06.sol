
        contract c {
            event foo (bool x, uint32 y, address x);
        }
// ---- Expect: diagnostics ----
// error: 3:50-51: event 'foo' has duplicate field name 'x'
// 	note 3:24-30: location of previous declaration of 'x'
