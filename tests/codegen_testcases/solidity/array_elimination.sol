// RUN: --target solana --emit cfg
contract TestCase {
    int128[] st;

    // BEGIN-CHECK: TestCase::TestCase::function::myFuncStorage__uint32
    function myFuncStorage(uint32[] storage arr) private {
        // CHECK: pop storage ty:uint32 slot(%arr)
        arr.pop();
    }
    
    // BEGIN-CHECK: TestCase::TestCase::function::myFuncStorage2__uint32
    function myFuncStorage2(uint32[] storage arr) private {
        // CHECK: push storage ty:uint32 slot:%arr = uint32 5
        arr.push(5);
    }

    // BEGIN-CHECK: TestCase::TestCase::function::myPush__int128
    function myPush(int128 val) public {
        // CHECK: push storage ty:int128 slot:uint32 16 = (arg #0)
        st.push(val);
    }

    // BEGIN-CHECK: TestCase::TestCase::function::myPop
    function myPop() public {
        // CHECK: pop storage ty:int128 slot(uint32 16)
        st.pop();
    }

    // BEGIN-CHECK: TestCase::TestCase::function::myFuncPointer__int256
    function myFuncPointer(int256[] memory arr) pure private {
        // CHECK: push array ty:int256[] value:int256 256
        arr.push(256);
        int256[] memory myArr = arr;
        myArr.push(5);
    }

    // BEGIN-CHECK: TestCase::TestCase::function::myFuncPointer2__int256
    function myFuncPointer2(int256[] arr) pure private {
        // CHECK: pop array ty:int256[]
        arr.pop();
    }


    // BEGIN-CHECK: TestCase::TestCase::function::foo
    function foo() public pure returns (bytes memory) {
        bytes b1 = hex"41";
        // NOT-CHECK: ty:bytes %b2
        bytes b2 = hex"41";
        // NOT-CHECK: ty:bytes %b3
        bytes b3 = hex"caffee";
        bytes b4 = hex"77ea";        

        // NOT-CHECK: push array ty:bytes value:bytes1 65
        b2.push(0x41);
        // NOT-CHECK: pop array ty:bytes
        b3.pop();

        // NOT-CHECK: builtin ArrayLength (%b4)
        uint32 c = b4.length;

        return (b1);
    }

    // BEGIN-CHECK: TestCase::TestCase::function::refPop__uint32
    function refPop(uint32[] memory arrMem) public {
        int128[] storage str_ref = st;
        // CHECK: push storage ty:int128 slot:%str_ref = int128 3
        str_ref.push(3);

        uint32[] memory ptr = arrMem;

        // CHECK: push array ty:uint32[] value:uint32 54
        ptr.push(54);
    }

    // BEGIN-CHECK: TestCase::TestCase::function::ohterTest
    function ohterTest() public pure returns (int24[] memory) {
        int24[] memory a1;

        int24[] memory ptr = a1;
        // CHECK: pop array ty:int24[]
        ptr.pop();

        return a1;
    }
}