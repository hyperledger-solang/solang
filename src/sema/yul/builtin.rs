use phf::{phf_map, phf_set};

pub struct YulBuiltinPrototype {
    pub name: &'static str,
    pub no_args: u8,
    pub no_returns: u8,
    pub doc: &'static str,
    pub ty: YulBuiltInFunction,
    pub stops_execution: bool,
}

// The enums declaration order should match that of the static vector containing the builtins
#[derive(Clone, Debug, PartialEq, Copy)]
#[repr(u8)]
pub enum YulBuiltInFunction {
    Stop = 0,
    Add = 1,
    Sub = 2,
    Mul = 3,
    Div = 4,
    SDiv = 5,
    Mod = 6,
    SMod = 7,
    Exp = 8,
    Not = 9,
    Lt = 10,
    Gt = 11,
    Slt = 12,
    Sgt = 13,
    Eq = 14,
    IsZero = 15,
    And = 16,
    Or = 17,
    Xor = 18,
    Byte = 19,
    Shl = 20,
    Shr = 21,
    Sar = 22,
    AddMod = 23,
    MulMod = 24,
    SignExtend = 25,
    Keccak256 = 26,
    Pc = 27,
    Pop = 28,
    MLoad = 29,
    MStore = 30,
    MStore8 = 31,
    SLoad = 32,
    SStore = 33,
    MSize = 34,
    Gas = 35,
    Address = 36,
    Balance = 37,
    SelfBalance = 38,
    Caller = 39,
    CallValue = 40,
    CallDataLoad = 41,
    CallDataSize = 42,
    CallDataCopy = 43,
    CodeSize = 44,
    CodeCopy = 45,
    ExtCodeSize = 46,
    ExtCodeCopy = 47,
    ReturnDataSize = 48,
    ReturnDataCopy = 49,
    ExtCodeHash = 50,
    Create = 51,
    Create2 = 52,
    Call = 53,
    CallCode = 54,
    DelegateCall = 55,
    StaticCall = 56,
    Return = 57,
    Revert = 58,
    SelfDestruct = 59,
    Invalid = 60,
    Log0 = 61,
    Log1 = 62,
    Log2 = 63,
    Log3 = 64,
    Log4 = 65,
    ChainId = 66,
    BaseFee = 67,
    Origin = 68,
    GasPrice = 69,
    BlockHash = 70,
    CoinBase = 71,
    Timestamp = 72,
    Number = 73,
    Difficulty = 74,
    GasLimit = 75,
}

// These are functions that do high level stuff in a contract and are not yet implemented.
static UNSUPPORTED_BUILTINS: phf::Set<&'static str> = phf_set! {
    "datasize", "dataoffset", "datacopy", "setimmutable", "loadimmutable",
    "linkersymbol", "memoryguard"
};

/// Checks if bultin function is unsupported
pub(crate) fn yul_unsupported_builtin(name: &str) -> bool {
    UNSUPPORTED_BUILTINS.contains(name)
}

static BUILTIN_YUL_FUNCTIONS: phf::Map<&'static str, YulBuiltInFunction> = phf_map! {
    "stop" => YulBuiltInFunction::Stop,
    "add" => YulBuiltInFunction::Add,
    "sub" => YulBuiltInFunction::Sub,
    "mul" => YulBuiltInFunction::Mul,
    "div" => YulBuiltInFunction::Div,
    "sdiv" => YulBuiltInFunction::SDiv,
    "mod" => YulBuiltInFunction::Mod,
    "smod" => YulBuiltInFunction::SMod,
    "exp" => YulBuiltInFunction::Exp,
    "not" => YulBuiltInFunction::Not,
    "lt" => YulBuiltInFunction::Lt,
    "gt" => YulBuiltInFunction::Gt,
    "slt" => YulBuiltInFunction::Slt,
    "sgt" => YulBuiltInFunction::Sgt,
    "eq" => YulBuiltInFunction::Eq,
    "iszero" => YulBuiltInFunction::IsZero,
    "and" => YulBuiltInFunction::And,
    "or" => YulBuiltInFunction::Or,
    "xor" => YulBuiltInFunction::Xor,
    "byte" => YulBuiltInFunction::Byte,
    "shl" => YulBuiltInFunction::Shl,
    "shr" => YulBuiltInFunction::Shr,
    "sar" => YulBuiltInFunction::Sar,
    "addmod" => YulBuiltInFunction::AddMod,
    "mulmod" => YulBuiltInFunction::MulMod,
    "signextend" => YulBuiltInFunction::SignExtend,
    "keccak256" => YulBuiltInFunction::Keccak256,
    "pc" => YulBuiltInFunction::Pc,
    "pop" => YulBuiltInFunction::Pop,
    "mload" => YulBuiltInFunction::MLoad,
    "mstore" => YulBuiltInFunction::MStore,
    "mstore8" => YulBuiltInFunction::MStore8,
    "sload" => YulBuiltInFunction::SLoad,
    "sstore" => YulBuiltInFunction::SStore,
    "msize" => YulBuiltInFunction::MSize,
    "gas" => YulBuiltInFunction::Gas,
    "address" => YulBuiltInFunction::Address,
    "balance" => YulBuiltInFunction::Balance,
    "selfbalance" => YulBuiltInFunction::SelfBalance,
    "caller" => YulBuiltInFunction::Caller,
    "callvalue" => YulBuiltInFunction::CallValue,
    "calldataload" => YulBuiltInFunction::CallDataLoad,
    "calldatasize" => YulBuiltInFunction::CallDataSize,
    "calldatacopy" => YulBuiltInFunction::CallDataCopy,
    "codesize" => YulBuiltInFunction::CodeSize,
    "codecopy" => YulBuiltInFunction::CodeCopy,
    "extcodesize" => YulBuiltInFunction::ExtCodeSize,
    "extcodecopy" => YulBuiltInFunction::ExtCodeCopy,
    "returndatasize" => YulBuiltInFunction::ReturnDataSize,
    "returndatacopy" => YulBuiltInFunction::ReturnDataCopy,
    "extcodehash" => YulBuiltInFunction::ExtCodeHash,
    "create" => YulBuiltInFunction::Create,
    "create2" => YulBuiltInFunction::Create2,
    "call" => YulBuiltInFunction::Call,
    "callcode" => YulBuiltInFunction::CallCode,
    "delegatecall" => YulBuiltInFunction::DelegateCall,
    "staticcall" => YulBuiltInFunction::StaticCall,
    "return" => YulBuiltInFunction::Return,
    "revert" => YulBuiltInFunction::Revert,
    "selfdestruct" => YulBuiltInFunction::SelfDestruct,
    "invalid" => YulBuiltInFunction::Invalid,
    "log0" => YulBuiltInFunction::Log0,
    "log1" => YulBuiltInFunction::Log1,
    "log2" => YulBuiltInFunction::Log2,
    "log3" => YulBuiltInFunction::Log3,
    "log4" => YulBuiltInFunction::Log4,
    "chainid" => YulBuiltInFunction::ChainId,
    "basefee" => YulBuiltInFunction::BaseFee,
    "origin" => YulBuiltInFunction::Origin,
    "gasprice" => YulBuiltInFunction::GasPrice,
    "blockhash" => YulBuiltInFunction::BlockHash,
    "coinbase" => YulBuiltInFunction::CoinBase,
    "timestamp" => YulBuiltInFunction::Timestamp,
    "number" => YulBuiltInFunction::Number,
    "difficulty" => YulBuiltInFunction::Difficulty,
    "gaslimit" => YulBuiltInFunction::GasLimit,
};

/// Retrieved the builtin function type from an identifier name
pub fn parse_builtin_keyword(keyword: &str) -> Option<&YulBuiltInFunction> {
    BUILTIN_YUL_FUNCTIONS.get(keyword)
}

impl YulBuiltInFunction {
    /// Retrieve the prototype from the enum type
    pub(crate) fn get_prototype_info(self) -> &'static YulBuiltinPrototype {
        let index = self as usize;
        &YUL_BUILTIN[index]
    }

    pub(crate) fn modify_state(&self) -> bool {
        matches!(
            self,
            YulBuiltInFunction::SStore
                | YulBuiltInFunction::Log0
                | YulBuiltInFunction::Log1
                | YulBuiltInFunction::Log2
                | YulBuiltInFunction::Log3
                | YulBuiltInFunction::Log4
                | YulBuiltInFunction::Create
                | YulBuiltInFunction::Call
                | YulBuiltInFunction::CallCode
                | YulBuiltInFunction::DelegateCall
                | YulBuiltInFunction::Create2
                | YulBuiltInFunction::SelfDestruct
        )
    }

    pub(crate) fn read_state(&self) -> bool {
        matches!(
            self,
            YulBuiltInFunction::Address
                | YulBuiltInFunction::SelfBalance
                | YulBuiltInFunction::Balance
                | YulBuiltInFunction::Origin
                | YulBuiltInFunction::Caller
                | YulBuiltInFunction::CallValue
                | YulBuiltInFunction::ChainId
                | YulBuiltInFunction::BaseFee
                | YulBuiltInFunction::Gas
                | YulBuiltInFunction::GasPrice
                | YulBuiltInFunction::ExtCodeSize
                | YulBuiltInFunction::ExtCodeCopy
                | YulBuiltInFunction::ExtCodeHash
                | YulBuiltInFunction::BlockHash
                | YulBuiltInFunction::CoinBase
                | YulBuiltInFunction::Timestamp
                | YulBuiltInFunction::Number
                | YulBuiltInFunction::Difficulty
                | YulBuiltInFunction::GasLimit
                | YulBuiltInFunction::StaticCall
                | YulBuiltInFunction::SLoad
        )
    }
}

impl ToString for YulBuiltInFunction {
    fn to_string(&self) -> String {
        let prototype = self.get_prototype_info();
        prototype.name.to_owned()
    }
}

// Yul built-in functions.
// Descriptions copied and slightly modified from: https://docs.soliditylang.org/en/v0.8.12/yul.html
static YUL_BUILTIN: [YulBuiltinPrototype; 76] =
    [
        YulBuiltinPrototype {
            name: "stop",
            no_args: 0,
            no_returns: 0,
            doc: "Stop execution",
            ty: YulBuiltInFunction::Stop,
            stops_execution: true,
        },
        YulBuiltinPrototype {
            name: "add",
            no_args: 2,
            no_returns: 1,
            doc: "add(x, y) returns x + y",
            ty: YulBuiltInFunction::Add,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sub",
            no_args: 2,
            no_returns: 1,
            doc: "sub(x, y) returns x - y",
            ty: YulBuiltInFunction::Sub,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mul",
            no_args: 2,
            no_returns: 1,
            doc: "mul(x, y) returns x*y",
            ty: YulBuiltInFunction::Mul,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "div",
            no_args: 2,
            no_returns: 1,
            doc: "div(x, y) returns x/y or 0 if y == 0",
            ty: YulBuiltInFunction::Div,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sdiv",
            no_args: 2,
            no_returns: 1,
            doc: "sdiv(x, y) returns x/y or 0 if y==0. Used for signed numbers in two's complement",
            ty: YulBuiltInFunction::SDiv,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mod",
            no_args: 2,
            no_returns: 1,
            doc: "mod(x, y) returns x % y or 0 if y == 0",
            ty: YulBuiltInFunction::Mod,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "smod",
            no_args: 2,
            no_returns: 1,
            doc: "smod(x, y) returns x % y or 0 if y == 0. Used for signed numbers in two's complement",
            ty: YulBuiltInFunction::SMod,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "exp",
            no_args: 2,
            no_returns: 1,
            doc: "exp(x, y) returns x to the power of y",
            ty: YulBuiltInFunction::Exp,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "not",
            no_args: 1,
            no_returns: 1,
            doc: "not(x): bitwise \"not\" of x (every bit is negated)",
            ty: YulBuiltInFunction::Not,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "lt",
            no_args: 2,
            no_returns: 1,
            doc: "lt(x, y) returns 1 if x < y, 0 otherwise",
            ty: YulBuiltInFunction::Lt,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "gt",
            no_args: 2,
            no_returns: 1,
            doc: "gt(x, y) returns 1 if x > y, 0 otherwise",
            ty: YulBuiltInFunction::Gt,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "slt",
            no_args: 2,
            no_returns: 1,
            doc: "slt(x, y) returns 1 if x > y, 0 otherwise. Used for signed numbers in two's complement",
            ty: YulBuiltInFunction::Slt,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sgt",
            no_args: 2,
            no_returns: 1,
            doc: "sgt(x, y) returns 1 if x > y, 0 otherwise. Used for signed numbers in two's complement",
            ty: YulBuiltInFunction::Sgt,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "eq",
            no_args: 2,
            no_returns: 1,
            doc: "eq(x, y) returns 1 if x == y, 0 otherwise",
            ty: YulBuiltInFunction::Eq,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "iszero",
            no_args: 1,
            no_returns: 1,
            doc: "iszero(x) returns 1 if x == 0, 0 otherwise",
            ty: YulBuiltInFunction::IsZero,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "and",
            no_args: 2,
            no_returns: 1,
            doc: "and(x, y) returns the bitwise \"and\" between x and y",
            ty: YulBuiltInFunction::And,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "or",
            no_args: 2,
            no_returns: 1,
            doc: "or(x, y) returns the bitwise \"or\" between x and y",
            ty: YulBuiltInFunction::Or,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "xor",
            no_args: 2,
            no_returns: 1,
            doc: "xor(x, y) returns the bitwise \"xor\" between x and y",
            ty: YulBuiltInFunction::Xor,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "byte",
            no_args: 2,
            no_returns: 1,
            doc: "byte(n, x) returns the nth byte of x, where the most significant byt is the 0th",
            ty: YulBuiltInFunction::Byte,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "shl",
            no_args: 2,
            no_returns: 1,
            doc: "shl(x, y) returns the logical shift left of y by x bits",
            ty: YulBuiltInFunction::Shl,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "shr",
            no_args: 2,
            no_returns: 1,
            doc: "shr(x, y) returns the logical shift right of y by x bits",
            ty: YulBuiltInFunction::Shr,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sar",
            no_args: 2,
            no_returns: 1,
            doc: "signed arithmetic shift right y by x bits",
            ty: YulBuiltInFunction::Sar,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "addmod",
            no_args: 3,
            no_returns: 1,
            doc: "addmod(x, y, m) returns (x + y) % m or 0 if m == 0",
            ty: YulBuiltInFunction::AddMod,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mulmod",
            no_args: 3,
            no_returns: 1,
            doc: "mulmod(x, y, m) returns (x * y) % m or 0 if m == 0",
            ty: YulBuiltInFunction::MulMod,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "signextend",
            no_args: 2,
            no_returns: 1,
            doc: "signextend(i, x) sign extends from (i*8+7)th bit counting from least significant",
            ty: YulBuiltInFunction::SignExtend,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "keccak256",
            no_args: 2,
            no_returns: 1,
            doc: "keccak256(p, n) performs keccak(mem[p...(p+n)])",
            ty: YulBuiltInFunction::Keccak256,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "pc",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the current position in code, i.e. the program counter",
            ty: YulBuiltInFunction::Pc,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "pop",
            no_args: 1,
            no_returns: 0,
            doc: "pop(x) discard value x",
            ty: YulBuiltInFunction::Pop,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mload",
            no_args: 1,
            no_returns: 1,
            doc: "mload(p) returns mem[p...(p+32)]",
            ty: YulBuiltInFunction::MLoad,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mstore",
            no_args: 2,
            no_returns: 0,
            doc: "mstore(p, v) stores v into mem[p...(p+32)]",
            ty: YulBuiltInFunction::MStore,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "mstore8",
            no_args: 2,
            no_returns: 0,
            doc: "mstore8(p, v) stores (v & 0xff) into mem[p] (modified a single byte of v)",
            ty: YulBuiltInFunction::MStore8,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sload",
            no_args: 1,
            no_returns: 1,
            doc: "sload(p) returns storage[p], i.e. memory on contract's storage",
            ty: YulBuiltInFunction::SLoad,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "sstore",
            no_args: 2,
            no_returns: 0,
            doc: "sstore(p) stores v into storage[p]",
            ty: YulBuiltInFunction::SStore,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "msize",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the size of memory, i.e largest accessed memory index",
            ty: YulBuiltInFunction::MSize,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "gas",
            no_args: 0,
            no_returns: 1,
            doc: "Returns gas still available to execution",
            ty: YulBuiltInFunction::Gas,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "address",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the address of the current contract / execution context",
            ty: YulBuiltInFunction::Address,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "balance",
            no_args: 1,
            no_returns: 1,
            doc: "balance(a) returns the wei balance at address a",
            ty: YulBuiltInFunction::Balance,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "selfbalance",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the wei balance at the address of the current contract / execution context",
            ty: YulBuiltInFunction::SelfBalance,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "caller",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the call sender",
            ty: YulBuiltInFunction::Caller,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "callvalue",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the wei sent together with the current call",
            ty: YulBuiltInFunction::CallValue,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "calldataload",
            no_args: 1,
            no_returns: 1,
            doc: "calldataload(p) returns call data starting from position p (32 bytes)",
            ty: YulBuiltInFunction::CallDataLoad,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "calldatasize",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the size of call data in bytes",
            ty: YulBuiltInFunction::CallDataSize,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "calldatacopy",
            no_args: 3,
            no_returns: 0,
            doc: "calldatacopy(t, f, s) copies s bytes from calldata at position f to mem at position t",
            ty: YulBuiltInFunction::CallDataCopy,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "codesize",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the size of the current contract / execution context",
            ty: YulBuiltInFunction::CodeSize,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "codecopy",
            no_args: 3,
            no_returns: 0,
            doc: "codecopy(t, f, s) copies s bytes from code at position f to mem at position t",
            ty: YulBuiltInFunction::CodeCopy,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "extcodesize",
            no_args: 1,
            no_returns: 1,
            doc: "extcodesize(a) returns the size of the code at address a",
            ty: YulBuiltInFunction::ExtCodeSize,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "extcodecopy",
            no_args: 4,
            no_returns: 0,
            doc: "extcodecopy(a, t, f, s) copies s bytes from code located at address a at position f to mem at position t",
            ty: YulBuiltInFunction::ExtCodeCopy,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "returndatasize",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the size of the last returndata",
            ty: YulBuiltInFunction::ReturnDataSize,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "returndatacopy",
            no_args: 3,
            no_returns: 0,
            doc: "returndatacopy(t, f, s) copy s bytes from return data at position f to mem at position t",
            ty: YulBuiltInFunction::ReturnDataCopy,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "extcodehash",
            no_args: 1,
            no_returns: 1,
            doc: "extcodehash(a) returns the code hash of address a",
            ty: YulBuiltInFunction::ExtCodeHash,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "create",
            no_args: 3,
            no_returns: 1,
            doc: "create(v, p, n) creates new contract with code mem[p..(p+n)] and sends v wei. It returns the new address or 0 on error",
            ty: YulBuiltInFunction::Create,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "create2",
            no_args: 4,
            no_returns: 1,
            doc: "create2(v, p, n, s) new contract with code mem[p...(p+n)] at address keccak256(0xff . this . s . keccak256(mem[p...(p+n)]) and sends v wei.\n 0xff is a 1 byte value, 'this' is the current contract's address as a 20 byte value and 's' is a big endian 256-bit value. it returns 0 on error.",
            ty: YulBuiltInFunction::Create2,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "call",
            no_args: 7,
            no_returns: 1,
            doc: "call(g, a, v, in, insize, out, outsize) calls contract at address a with input mem[in...(in+insize)] providing f cas and v wei and outputs area mem[out...(out+outsize)]. It returns 0 on error and 1 on success",
            ty: YulBuiltInFunction::Call,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "callcode",
            no_args: 7,
            no_returns: 1,
            doc: "Identical to call(g, a, v, in, insize, out, outsize), but only use the code from a and stay in the context of the current contract otherwise",
            ty: YulBuiltInFunction::CallCode,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "delegatecall",
            no_args: 6,
            no_returns: 1,
            doc: "Identical to 'callcode' but also keep caller and callvalue",
            ty: YulBuiltInFunction::DelegateCall,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "staticcall",
            no_args: 6,
            no_returns: 1,
            doc: "Identical to call(g, a, 0, in, insize, out, outsize), but do not allow state modifications",
            ty: YulBuiltInFunction::StaticCall,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "return",
            no_args: 2,
            no_returns: 0,
            doc: "return(p, s) ends execution and returns data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Return,
            stops_execution: true,
        },
        YulBuiltinPrototype {
            name: "revert",
            no_args: 2,
            no_returns: 0,
            doc: "revert(p, s) ends execution, reverts state changes and returns data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Revert,
            stops_execution: true,
        },
        YulBuiltinPrototype {
            name: "selfdestruct",
            no_args: 1,
            no_returns: 0,
            doc: "selfdestruct(a) ends execution, destroy current contract and sends funds to a",
            ty: YulBuiltInFunction::SelfDestruct,
            stops_execution: true,
        },
        YulBuiltinPrototype {
            name: "invalid",
            no_args: 0,
            no_returns: 0,
            doc: "Ends execution with invalid instruction",
            ty: YulBuiltInFunction::Invalid,
            stops_execution: true,
        },
        YulBuiltinPrototype {
            name: "log0",
            no_args: 2,
            no_returns: 0,
            doc: "log(p, s): log without topics and data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Log0,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "log1",
            no_args: 3,
            no_returns: 0,
            doc: "log1(p, s, t1): log with topic t1 and data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Log1,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "log2",
            no_args: 4,
            no_returns: 0,
            doc: "log2(p, s, t1, t2): log with topics t1, t2 and data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Log2,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "log3",
            no_args: 5,
            no_returns: 0,
            doc: "log3(p, s, t1, t2, t3): log with topics t1, t2, t3 and data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Log3,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "log4",
            no_args: 6,
            no_returns: 0,
            doc: "log4(p, s, t1, t2, t3, t4): log with topics t1, t2, t3, t4 with data mem[p...(p+s)]",
            ty: YulBuiltInFunction::Log4,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "chainid",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the ID of the executing chain",
            ty: YulBuiltInFunction::ChainId,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "basefee",
            no_args: 0,
            no_returns: 1,
            doc: "Return the current block's base fee",
            ty: YulBuiltInFunction::BaseFee,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "origin",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the transaction sender",
            ty: YulBuiltInFunction::Origin,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "gasprice",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the gas price of the transaction",
            ty: YulBuiltInFunction::GasPrice,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "blockhash",
            no_args: 1,
            no_returns: 1,
            doc: "blockhash(b) return the hash of block #b - only valid for the last 256 executing block excluding current",
            ty: YulBuiltInFunction::BlockHash,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "coinbase",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the current mining beneficiary",
            ty: YulBuiltInFunction::CoinBase,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "timestamp",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the timestamp of the current block in seconds since the epoch",
            ty: YulBuiltInFunction::Timestamp,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "number",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the current block's number",
            ty: YulBuiltInFunction::Number,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "difficulty",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the difficulty of the current block",
            ty: YulBuiltInFunction::Difficulty,
            stops_execution: false,
        },
        YulBuiltinPrototype {
            name: "gaslimit",
            no_args: 0,
            no_returns: 1,
            doc: "Returns the current block's gas limit",
            ty: YulBuiltInFunction::GasLimit,
            stops_execution: false,
        },
    ];

#[test]
fn test_builtin_indexes() {
    for item in &YUL_BUILTIN {
        let name = item.name;
        let ty = item.ty;

        let parsed_ty = parse_builtin_keyword(name).unwrap();
        assert_eq!(ty, *parsed_ty);
        assert_eq!(name, parsed_ty.get_prototype_info().name);
    }
}
