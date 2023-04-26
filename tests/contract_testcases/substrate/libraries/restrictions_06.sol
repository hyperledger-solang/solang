
        library c is x {
            fallback() internal {}
        }
// ----
// error (22-23): library 'c' cannot have a base contract
// error (22-23): 'x' not found
// error (38-57): fallback not allowed in a library
