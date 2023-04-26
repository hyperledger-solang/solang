abstract contract c {
    event foo(bool x, uint32 y, address indexed);
}

// ----
// error (54-69): indexed event fields must have a name on substrate
