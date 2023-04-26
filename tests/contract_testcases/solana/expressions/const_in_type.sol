
        contract x {
            uint public constant Y = 24;

            constructor(bytes32[Y] memory foo) {}
        }
// ----
// warning (106-109): function parameter 'foo' has never been read
