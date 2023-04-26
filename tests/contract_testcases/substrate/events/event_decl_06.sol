
        contract c {
            event foo (bool x, uint32 y, address x);
        }
// ----
// error (71-72): event 'foo' has duplicate field name 'x'
// 	note (45-51): location of previous declaration of 'x'
