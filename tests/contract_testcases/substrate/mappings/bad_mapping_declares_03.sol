
        contract c {
            mapping(int => address) data;
            mapping(data => address) data2;
        }
// ----
// error (84-88): 'data' is a contract variable
