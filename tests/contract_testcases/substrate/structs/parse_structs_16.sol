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

// ----
// error (193-207): struct 'ArrayTreeFixed' has infinite size
// 	note (218-239): recursive field 'Arr'
