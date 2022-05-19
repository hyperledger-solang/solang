// RUN: --target ewasm --emit cfg
contract  Ownable  {

// BEGIN-CHECK: Ownable::Ownable::function::_msgData
    function  _msgData()  internal  view  returns  (bytes  memory)  {
        // CHECK: return (builtin Calldata ())
        return msg.data;
    }
}