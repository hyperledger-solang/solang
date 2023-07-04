/// Tests various 
contract c {
    function add_pointer(bytes memory _returnData)
        public
        pure
        returns (bytes memory)
    {
        assembly {
            _returnData :=  add(_returnData, 0x04)
        }
	assembly {
	    _returnData := 1
	}
        return _returnData;
    }

    function uint32_as_ptr(bytes memory _returnData)
        public
        pure
        returns (bytes memory)
    {
	uint32 p;
        assembly {
            p :=  add(_returnData, 0x04)
	    _returnData := p
        }
        return _returnData;
    }

    function ptr_to_ptr(bytes memory _returnData)
        public
        pure
        returns (bytes memory)
    {
	uint32[] foo = new uint32[](2);
        assembly {
	    _returnData := foo
        }
        return _returnData;
    }


}

// ---- Expect: diagnostics ----
