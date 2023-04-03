// RUN: --target substrate --emit cfg

contract has_fallback_and_receive {
	// BEGIN-CHECK: Contract: has_fallback_and_receive
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.1 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.2 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.3 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.1 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.4 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.4
	// CHECK:         switch %selector.temp.4:
	// CHECK:                 case uint32 3576764294: goto block #3
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         branchcond (unsigned more %value.temp.2 > uint128 0), block5, block4
	// CHECK: block3: # msg_3_dispatch
	// CHECK:          = call has_fallback_and_receive::has_fallback_and_receive::constructor::861731d5 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # fallback
	// CHECK:          = call has_fallback_and_receive::has_fallback_and_receive::fallback 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block5: # receive
	// CHECK:          = call has_fallback_and_receive::has_fallback_and_receive::receive 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	fallback() external {}
	receive() payable external {}
}

contract has_fallback {
	// BEGIN-CHECK: Contract: has_fallback
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.5 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.6 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.7 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.5 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.8 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.8
	// CHECK:         switch %selector.temp.8:
	// CHECK:                 case uint32 3576764294: goto block #3
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         branchcond (unsigned more %value.temp.6 > uint128 0), block5, block4
	// CHECK: block3: # msg_2_dispatch
	// CHECK:          = call has_fallback::has_fallback::constructor::861731d5 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # fallback
	// CHECK:          = call has_fallback::has_fallback::fallback 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block5: # receive
	// CHECK:         return code: function selector invalid
	fallback() external {}
}

contract has_receive {
	// BEGIN-CHECK: Contract: has_receive
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.9 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.10 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.11 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.9 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.12 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.12
	// CHECK:         switch %selector.temp.12:
	// CHECK:                 case uint32 3576764294: goto block #3
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         branchcond (unsigned more %value.temp.10 > uint128 0), block5, block4
	// CHECK: block3: # msg_2_dispatch
	// CHECK:          = call has_receive::has_receive::constructor::861731d5 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # fallback
	// CHECK:         return code: function selector invalid
	// CHECK: block5: # receive
	// CHECK:          = call has_receive::has_receive::receive 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	receive() payable external {}
}


contract is_payable {
	// BEGIN-CHECK: Contract: is_payable
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.13 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.14 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.15 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.13 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.16 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.16
	// CHECK:         switch %selector.temp.16:
	// CHECK:                 case uint32 2018875586: goto block #3
	// CHECK:                 case uint32 2114960382: goto block #6
	// CHECK:                 case uint32 3576764294: goto block #7
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         return code: function selector invalid
	// CHECK: block3: # msg_0_dispatch
	// CHECK:         branchcond (unsigned more %value.temp.14 > uint128 0), block4, block5
	// CHECK: block4: # msg_0_has_value
	// CHECK:         assert-failure
	// CHECK: block5: # msg_0_no_value
	// CHECK:          = call is_payable::is_payable::function::foo 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block6: # msg_1_dispatch
	// CHECK:          = call is_payable::is_payable::function::bar 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block7: # msg_3_dispatch
	// CHECK:          = call is_payable::is_payable::constructor::861731d5 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	function foo() public pure {}
	function bar() public payable { require(msg.value > 0); }
}

contract overloaded {
	// BEGIN-CHECK: Contract: overloaded
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.17 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.18 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.19 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.17 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.22 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.22
	// CHECK:         switch %selector.temp.22:
	// CHECK:                 case uint32 2018875586: goto block #3
	// CHECK:                 case uint32 2114960382: goto block #4
	// CHECK:                 case uint32 4028568102: goto block #5
	// CHECK:                 case uint32 2338643635: goto block #6
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         branchcond (unsigned more %value.temp.18 > uint128 0), block14, block13
	// CHECK: block3: # msg_0_dispatch
	// CHECK:          = call overloaded::overloaded::constructor::c2985578 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # msg_1_dispatch
	// CHECK:          = call overloaded::overloaded::constructor::febb0f7e 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block5: # msg_2_dispatch
	// CHECK:          = call overloaded::overloaded::function::f 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block6: # msg_3_dispatch
	// CHECK:         branchcond (unsigned more %value.temp.18 > uint128 0), block7, block8
	// CHECK: block7: # msg_3_has_value
	// CHECK:         assert-failure
	// CHECK: block8: # msg_3_no_value
	// CHECK:         ty:uint32 %temp.20 = (trunc uint32 (%input_len.temp.17 - uint32 4))
	// CHECK:         branchcond (unsigned (uint32 0 + uint32 32) <= %temp.20), block9, block10
	// CHECK: block9: # inbounds
	// CHECK:         ty:uint256 %temp.21 = (builtin ReadFromBuffer ((advance ptr: %input_ptr.temp.19, by: uint32 4), uint32 0))
	// CHECK:         branchcond (unsigned less (uint32 0 + uint32 32) < %temp.20), block11, block12
	// CHECK: block10: # out_of_bounds
	// CHECK:         assert-failure
	// CHECK: block11: # not_all_bytes_read
	// CHECK:         assert-failure
	// CHECK: block12: # buffer_read
	// CHECK:          = call overloaded::overloaded::function::f__uint256 %temp.21
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block13: # fallback
	// CHECK:          = call overloaded::overloaded::fallback 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block14: # receive
	// CHECK:          = call overloaded::overloaded::receive 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	constructor foo() {}
	constructor bar() {}
	function f() public payable {}
	function f(uint256 i) public pure {}
	fallback() external {}
	receive() payable external {}
}

contract simple {
	// BEGIN-CHECK: Contract: simple
	// CHECK: block0: # entry
	// CHECK:         ty:uint32 %input_len.temp.23 = (arg #1)
	// CHECK:         ty:uint128 %value.temp.24 = (arg #2)
	// CHECK:         ty:buffer_pointer %input_ptr.temp.25 = (arg #0)
	// CHECK:         branchcond (unsigned less %input_len.temp.23 < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK:         ty:uint32 %selector.temp.26 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK:         store (arg #3), %selector.temp.26
	// CHECK:         switch %selector.temp.26:
	// CHECK:                 case uint32 2018875586: goto block #3
	// CHECK:                 case uint32 3576764294: goto block #6
	// CHECK:                 default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK:         return code: function selector invalid
	// CHECK: block3: # msg_0_dispatch
	// CHECK:         branchcond (unsigned more %value.temp.24 > uint128 0), block4, block5
	// CHECK: block4: # msg_0_has_value
	// CHECK:         assert-failure
	// CHECK: block5: # msg_0_no_value
	// CHECK:          = call simple::simple::function::foo 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block6: # msg_2_dispatch
	// CHECK:          = call simple::simple::constructor::861731d5 
	// CHECK:         return data (alloc bytes len uint32 0), data length: uint32 0
	function foo() public pure {}
}