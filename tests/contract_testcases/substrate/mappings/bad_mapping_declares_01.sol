
        contract c {
            mapping(uint[] => address) data;
        }
// ----
// error (42-48): key of mapping cannot be array type
