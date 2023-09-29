library StorageSlot {
    struct AddressSlot {
        address value;
    }

    function getAddressSlot(
        bytes32 slot
    ) internal pure returns (AddressSlot storage r) {
        assembly {
            r.slot := slot
        }
    }
}

contract StorageKeys {
    function get(bytes32 key) public view returns (address a) {
        a = StorageSlot.getAddressSlot(key).value;
    }
}

// ---- Expect: diagnostics ----
