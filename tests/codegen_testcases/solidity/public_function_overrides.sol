// RUN: --target polkadot --release --emit cfg

interface IERC165 {
    function supportsInterface(bytes4 interfaceId) external view returns (bool);
}

interface IERC1155 is IERC165 {}

// CHECK: # function ERC165::ERC165::function::supportsInterface__bytes4 public:true
contract ERC165 is IERC165 {
    function supportsInterface(
        bytes4 interfaceId
    ) public view virtual override returns (bool) {}
}

// CHECK: # function ERC1155::ERC165::function::supportsInterface__bytes4 public:false
// CHECK: # function ERC1155::ERC1155::function::supportsInterface__bytes4 public:true
contract ERC1155 is ERC165, IERC1155 {
    function supportsInterface(
        bytes4 interfaceId
    ) public view virtual override(ERC165, IERC165) returns (bool) {}
    // CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
    // CHECK: case uint32 3576764294

    // CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
    // CHECK: case uint32 2815033089
    // NOT-CHECK: case uint32 3576764294
}

contract MyToken is ERC1155 {}
