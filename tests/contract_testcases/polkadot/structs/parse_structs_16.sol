abstract contract AddressTree {
    struct MapTree {
        mapping(uint256 => MapTree) Items; // OK
    }
    struct ArrayTreeDynamic {
        ArrayTreeDynamic[] Arr; // OK
    }
    struct ArrayTreeFixed {
        ArrayTreeFixed[2] Arr; // Recursive struct definision
    }
}

// ---- Expect: diagnostics ----
// error: 8:12-26: struct 'ArrayTreeFixed' has infinite size
// 	note 9:9-30: recursive field 'Arr'
