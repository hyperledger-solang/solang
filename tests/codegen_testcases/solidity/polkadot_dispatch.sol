// RUN: --target polkadot --emit cfg

contract has_fallback_and_receive {
	// BEGIN-CHECK: Contract: has_fallback_and_receive

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.1 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.2 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.3 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.4 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.4
	// CHECK: 	switch %selector.temp.4:
	// CHECK: 		case uint32 3576764294: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_3_dispatch
	// CHECK: 	 = call has_fallback_and_receive::has_fallback_and_receive::constructor::861731d5 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.5 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.6 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.7 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.8 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.8
	// CHECK: 	switch %selector.temp.8:
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block4, block3
	// CHECK: block3: # fallback
	// CHECK: 	 = call has_fallback_and_receive::has_fallback_and_receive::fallback 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # receive
	// CHECK: 	 = call has_fallback_and_receive::has_fallback_and_receive::receive 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	fallback() external {}
	receive() payable external {}
}

contract has_fallback {
	// BEGIN-CHECK: Contract: has_fallback

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.9 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.10 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.11 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.12 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.12
	// CHECK: 	switch %selector.temp.12:
	// CHECK: 		case uint32 3576764294: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_2_dispatch
	// CHECK: 	 = call has_fallback::has_fallback::constructor::861731d5 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.13 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.14 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.15 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.16 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.16
	// CHECK: 	switch %selector.temp.16:
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block4, block3
	// CHECK: block3: # fallback
	// CHECK: 	 = call has_fallback::has_fallback::fallback 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # receive
	// CHECK: 	return code: function selector invalid

	fallback() external {}
}

contract has_receive {
	// BEGIN-CHECK: Contract: has_receive

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.17 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.18 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.19 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.20 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.20
	// CHECK: 	switch %selector.temp.20:
	// CHECK: 		case uint32 3576764294: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_2_dispatch
	// CHECK: 	 = call has_receive::has_receive::constructor::861731d5 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.21 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.22 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.23 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.24 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.24
	// CHECK: 	switch %selector.temp.24:
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block4, block3
	// CHECK: block3: # fallback
	// CHECK: 	return code: function selector invalid
	// CHECK: block4: # receive
	// CHECK: 	 = call has_receive::has_receive::receive 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	receive() payable external {}
}


contract is_payable {
	// BEGIN-CHECK: Contract: is_payable

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.25 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.26 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.27 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.28 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.28
	// CHECK: 	switch %selector.temp.28:
	// CHECK: 		case uint32 3576764294: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_3_dispatch
	// CHECK: 	 = call is_payable::is_payable::constructor::861731d5 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.29 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.30 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.31 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.32 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.32
	// CHECK: 	switch %selector.temp.32:
	// CHECK: 		case uint32 2018875586: goto block #3
	// CHECK: 		case uint32 2114960382: goto block #6
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_0_dispatch
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block4, block5
	// CHECK: block4: # func_0_got_value
	// CHECK: 	assert-failure
	// CHECK: block5: # func_0_no_value
	// CHECK: 	 = call is_payable::is_payable::function::foo 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block6: # func_1_dispatch
	// CHECK: 	 = call is_payable::is_payable::function::bar 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	
	function foo() public pure {}
	function bar() public payable { require(msg.value > 0); }
}

contract overloaded {
	// BEGIN-CHECK: Contract: overloaded

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.33 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.34 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.35 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.36 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.36
	// CHECK: 	switch %selector.temp.36:
	// CHECK: 		case uint32 2018875586: goto block #3
	// CHECK: 		case uint32 2114960382: goto block #4
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_0_dispatch
	// CHECK: 	 = call overloaded::overloaded::constructor::c2985578 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # func_1_dispatch
	// CHECK: 	 = call overloaded::overloaded::constructor::febb0f7e 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.37 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.38 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.39 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.42 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.42
	// CHECK: 	switch %selector.temp.42:
	// CHECK: 		case uint32 4028568102: goto block #3
	// CHECK: 		case uint32 2338643635: goto block #4
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block12, block11
	// CHECK: block3: # func_2_dispatch
	// CHECK: 	 = call overloaded::overloaded::function::f 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block4: # func_3_dispatch
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block5, block6
	// CHECK: block5: # func_3_got_value
	// CHECK: 	assert-failure
	// CHECK: block6: # func_3_no_value
	// CHECK: 	branchcond (unsigned uint32 32 <= (trunc uint32 ((arg #1) - uint32 4))), block7, block8
	// CHECK: block7: # inbounds
	// CHECK: 	ty:uint256 %temp.41 = (builtin ReadFromBuffer ((advance ptr: %input_ptr.temp.39, by: uint32 4), uint32 0))
	// CHECK: 	branchcond (unsigned less uint32 32 < (trunc uint32 ((arg #1) - uint32 4))), block9, block10
	// CHECK: block8: # out_of_bounds
	// CHECK: 	assert-failure
	// CHECK: block9: # not_all_bytes_read
	// CHECK: 	assert-failure
	// CHECK: block10: # buffer_read
	// CHECK: 	 = call overloaded::overloaded::function::f__uint256 %temp.41
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block11: # fallback
	// CHECK: 	 = call overloaded::overloaded::fallback 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0
	// CHECK: block12: # receive
	// CHECK: 	 = call overloaded::overloaded::receive 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	constructor foo() payable {}
	constructor bar() payable {}
	function f() public payable {}
	function f(uint256 i) public pure {}
	fallback() external {}
	receive() payable external {}
}

contract simple {
	// BEGIN-CHECK: Contract: simple

	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.43 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.44 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.45 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.46 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.46
	// CHECK: 	switch %selector.temp.46:
	// CHECK: 		case uint32 3576764294: goto block #3
	// NOT-CHECK: 	case uint32 2018875586: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_2_dispatch
	// CHECK: 	 = call simple::simple::constructor::861731d5 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	// CHECK: # function polkadot_call_dispatch public:false selector: nonpayable:false
	// CHECK: # params: buffer_pointer,uint32,uint128,uint32
	// CHECK: # returns: 
	// CHECK: block0: # entry
	// CHECK: 	ty:uint32 %input_len.temp.47 = (arg #1)
	// CHECK: 	ty:uint128 %value.temp.48 = (arg #2)
	// CHECK: 	ty:buffer_pointer %input_ptr.temp.49 = (arg #0)
	// CHECK: 	branchcond (unsigned less (arg #1) < uint32 4), block2, block1
	// CHECK: block1: # start_dispatch
	// CHECK: 	ty:uint32 %selector.temp.50 = (builtin ReadFromBuffer ((arg #0), uint32 0))
	// CHECK: 	store (arg #3), %selector.temp.50
	// CHECK: 	switch %selector.temp.50:
	// CHECK: 		case uint32 2018875586: goto block #3
	// NOT-CHECK: 	case uint32 3576764294: goto block #3
	// CHECK: 		default: goto block #2
	// CHECK: block2: # fb_or_recv
	// CHECK: 	return code: function selector invalid
	// CHECK: block3: # func_0_dispatch
	// CHECK: 	branchcond (unsigned more (arg #2) > uint128 0), block4, block5
	// CHECK: block4: # func_0_got_value
	// CHECK: 	assert-failure
	// CHECK: block5: # func_0_no_value
	// CHECK: 	 = call simple::simple::function::foo 
	// CHECK: 	return data (alloc bytes len uint32 0), data length: uint32 0

	function foo() public pure {}
}

contract nonpayableConstructor {
	// BEGIN-CHECK: Contract: nonpayableConstructor
	
	// CHECK: # function polkadot_deploy_dispatch public:false selector: nonpayable:false

	// CHECK: switch %selector.temp.54:
    // CHECK: case uint32 2371928013: goto block #3

	// CHECK: block3: # func_0_dispatch
	// CHCEK: branchcond (unsigned more %value.temp.52 > uint128 0), block4, block5

	// CHECK: block4: # func_0_got_value
 	// CHECK: print 
 	// CHECK: assert-failure
	// CHECK: block5: # func_0_no_value
 	// CHECK: = call nonpayableConstructor::nonpayableConstructor::constructor::cdbf608d 
	
	constructor () {}
	function foo() public pure {}
}