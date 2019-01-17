use ast::*;
use std::ptr::null_mut;
use std::ffi::{CString, CStr};
use std::collections::HashMap;
use std::str;
use vartable::*;
use std::cell::Cell;
use std::slice;
use link;
use std::io::prelude::*;
use std::fs::File;

use llvm_sys::LLVMIntPredicate;
use llvm_sys::core::*;
use llvm_sys::prelude::*;
use llvm_sys::target::*;
use llvm_sys::target_machine::*;

const TRIPLE: &'static [u8] = b"wasm32-unknown-unknown-wasm\0";

const LLVM_FALSE: LLVMBool = 0;
const LLVM_TRUE: LLVMBool = 1;

lazy_static::lazy_static! {
    static ref LLVM_INIT: () = unsafe {
        LLVMInitializeWebAssemblyTargetInfo();
        LLVMInitializeWebAssemblyTarget();
        LLVMInitializeWebAssemblyTargetMC();
        LLVMInitializeWebAssemblyAsmPrinter();
        LLVMInitializeWebAssemblyAsmParser();
        LLVMInitializeWebAssemblyDisassembler();
    };
}

fn target_machine() -> LLVMTargetMachineRef {
    let mut target = null_mut();
    let mut err_msg_ptr = null_mut();
    unsafe {
        if LLVMGetTargetFromTriple(TRIPLE.as_ptr() as *const _, &mut target, &mut err_msg_ptr) == LLVM_TRUE {
            let err_msg_cstr = CStr::from_ptr(err_msg_ptr as *const _);
            let err_msg = str::from_utf8(err_msg_cstr.to_bytes()).unwrap();
            panic!("failed to create llvm target: {}", err_msg);
        }
    }

    unsafe {
        LLVMCreateTargetMachine(target,
                                TRIPLE.as_ptr() as *const _,
                                b"generic\0".as_ptr() as *const _,
                                b"\0".as_ptr() as *const _,
                                LLVMCodeGenOptLevel::LLVMCodeGenLevelAggressive,
                                LLVMRelocMode::LLVMRelocDefault,
                                LLVMCodeModel::LLVMCodeModelDefault)
    }
}

pub struct Contract {
    pub name: String,
    pub module: LLVMModuleRef,
}

impl Contract {
    pub fn dump_llvm(&self) {
        unsafe {
            LLVMDumpModule(self.module);
        }
    }

    pub fn wasm_file(&self, emitter: &Emitter, filename: String) -> Result<(), String> {
        let mut obj_error = null_mut();
        let mut memory_buffer = null_mut();

        unsafe {
            let result = LLVMTargetMachineEmitToMemoryBuffer(emitter.tm,
                                                    self.module,
                                                    LLVMCodeGenFileType::LLVMObjectFile,
                                                    &mut obj_error,
                                                    &mut memory_buffer);

            if result != 0 {
                Err(CStr::from_ptr(obj_error as *const _).to_string_lossy().to_string())
            } else {
                let obj = slice::from_raw_parts(
                    LLVMGetBufferStart(memory_buffer) as *const u8,
                    LLVMGetBufferSize(memory_buffer) as usize
                );
                let res = link::link(&obj);
                LLVMDisposeMemoryBuffer(memory_buffer);
                
                let mut file = File::create(filename).unwrap();
                file.write_all(&res).unwrap();
                Ok(())
            }
        }
    }

    #[cfg(test)]
    pub fn wasm(&self, emitter: &Emitter) -> Result<Vec<u8>, String> {
        let mut obj_error = null_mut();
        let mut memory_buffer = null_mut();

        unsafe {
            let result = LLVMTargetMachineEmitToMemoryBuffer(emitter.tm,
                                                    self.module,
                                                    LLVMCodeGenFileType::LLVMObjectFile,
                                                    &mut obj_error,
                                                    &mut memory_buffer);

            if result != 0 {
                Err(CStr::from_ptr(obj_error as *const _).to_string_lossy().to_string())
            } else {
                let obj = slice::from_raw_parts(
                    LLVMGetBufferStart(memory_buffer) as *const u8,
                    LLVMGetBufferSize(memory_buffer) as usize
                );
                let res = link::link(&obj);
                LLVMDisposeMemoryBuffer(memory_buffer);
                Ok(res)
            }
        }
    }
}

pub struct Emitter {
    context: LLVMContextRef,
    tm: LLVMTargetMachineRef,
    pub contracts: Vec<Contract>,
}

impl Emitter {
    pub fn new(s: SourceUnit) -> Self {
        lazy_static::initialize(&LLVM_INIT);

        let mut e = Emitter{
            context: unsafe { LLVMContextCreate() },
            tm: target_machine(),
            contracts: Vec::new()
        };

        for part in &s.parts {
            if let SourceUnitPart::ContractDefinition(ref contract) = part {
                let contractname = CString::new(contract.name.name.to_string()).unwrap();

                unsafe {
                    let module = LLVMModuleCreateWithName(contractname.as_ptr());
                    LLVMSetTarget(module, TRIPLE.as_ptr() as *const _);
                    LLVMSetSourceFileName(module, s.name.as_ptr() as *const _, s.name.len() as _);
                    let mut builder = LLVMCreateBuilderInContext(e.context);

                    for m in &contract.parts {
                        if let ContractPart::FunctionDefinition(ref func) = m {
                            if let Err(s) = emit_func(func, e.context, module, builder) {
                                println!("failed to compile: {}", s);
                            }
                        }
                    }

                    e.contracts.push(Contract{
                        name: contract.name.name.to_string(),
                        module: module,
                    });

                    LLVMDisposeBuilder(builder);
                }
            }

        }

        e
    }
}

impl Drop for Emitter {
    fn drop(&mut self) {
        for c in &self.contracts {
            unsafe {
                LLVMDisposeModule(c.module);
            }
        }

        unsafe {
            LLVMContextDispose(self.context);
            LLVMDisposeTargetMachine(self.tm);
        }
    }
}

unsafe fn emit_func(f: &FunctionDefinition, context: LLVMContextRef, module: LLVMModuleRef, builder: LLVMBuilderRef) -> Result<(), String> {
    let mut args = vec!();

    for p in &f.params {
        args.push(p.typ.LLVMType(context));
    }

    let fname = match f.name {
        None => {
            return Err("function with no name are not implemented yet".to_string());
        },
        Some(ref n) => {
            CString::new(n.name.to_string()).unwrap()
        }
    };

    let ret = match f.returns.len() {
        0 => LLVMVoidType(),
        1 => f.returns[0].typ.LLVMType(context),
        _ => return Err("only functions with one return value implemented".to_string())
    };

    let ftype = LLVMFunctionType(ret, args.as_mut_ptr(), args.len() as _, 0);

    let function = LLVMAddFunction(module, fname.as_ptr(), ftype);

    let bb = LLVMAppendBasicBlockInContext(context, function, b"entry\0".as_ptr() as *const _);

    LLVMPositionBuilderAtEnd(builder, bb);

    let mut emitter = FunctionEmitter{
        context: context,
        builder: builder,
        vartable: Vartable::new(),
        basic_block: bb,
        llfunction: function,
        function: &f,
        loop_scope: Vec::new(),
    };

    // create variable table
    if let Some(ref vartable) = f.vartable {
        for (name, typ) in vartable {
            emitter.vartable.insert(name, *typ, LLVMConstInt(typ.LLVMType(context), 0, LLVM_FALSE));
        }
    }

    let mut i = 0;
    for p in &f.params {
        // Unnamed function arguments are not accessible
        if let Some(ref argname) = p.name {
            emitter.vartable.insert(&argname.name, p.typ, LLVMGetParam(function, i));
        }
        i += 1;
    }

    f.body.visit_stmt(&mut |s| {
        if let Statement::VariableDefinition(v, Some(e)) = s {
            let value = emitter.expression(e, v.typ)?;

            emitter.vartable.insert(&(v.name.name), v.typ, value);
        }
        Ok(())
    })?;

    emitter.statement(&f.body)?;

    Ok(())
}

impl ElementaryTypeName {
    #[allow(non_snake_case)]
    fn LLVMType(&self, context: LLVMContextRef) -> LLVMTypeRef {
        match self {
            ElementaryTypeName::Bool => unsafe { LLVMInt1TypeInContext(context) },
            ElementaryTypeName::Int(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            ElementaryTypeName::Uint(n) => unsafe { LLVMIntTypeInContext(context, *n as _) },
            ElementaryTypeName::Address => unsafe { LLVMIntTypeInContext(context, 20*8) },
            _ => {
                panic!("llvm type for {:?} not implemented", self);
            }
        }
    }
}

struct FunctionEmitter<'a> {
    context: LLVMContextRef,
    builder: LLVMBuilderRef,
    llfunction: LLVMValueRef,
    basic_block: LLVMBasicBlockRef,
    function: &'a FunctionDefinition,
    vartable: Vartable,
    loop_scope: Vec<LoopScope>,
}

struct BasicBlock {
    pub basic_block: LLVMBasicBlockRef,
    pub phi: HashMap<String, LLVMValueRef>,
}

struct LoopScope {
    pub break_bb: BasicBlock,
    pub breaks_in_loop: Cell<u32>
}

impl<'a> FunctionEmitter<'a> {
    fn statement(&mut self, stmt: &Statement) -> Result<bool, String> {
        match stmt {
            Statement::VariableDefinition(_, _) => {
                // variables
            },
            Statement::BlockStatement(block) => {
                let mut reach = true;
                for st in &block.0 {
                    reach = self.statement(st)?;
                    if !reach {
                        break;
                    }
                }
                return Ok(reach);
            },
            Statement::Return(None) => {
                unsafe {
                    LLVMBuildRetVoid(self.builder);
                }
                return Ok(false);
            }
            Statement::Return(Some(expr)) => {
                let v = self.expression(expr, self.function.returns[0].typ)?;

                unsafe {
                    LLVMBuildRet(self.builder, v);
                }
                return Ok(false);
            },
            Statement::Expression(expr) => {
                self.expression(expr, ElementaryTypeName::Any)?;
            }
            Statement::Empty => {
                // nop
            },
            Statement::If(cond, then, box None) => {
                let mut changeset = HashMap::new();

                then.written_vars(&mut changeset);

                let then_bb = self.new_basic_block("then".to_string(), None);

                let endif_bb = self.new_basic_block("endif".to_string(), Some(&changeset));

                self.add_incoming(&endif_bb);

                let v = self.expression(cond, ElementaryTypeName::Bool)?;

                unsafe {
                    LLVMBuildCondBr(self.builder, v, then_bb.basic_block, endif_bb.basic_block);
                }

                self.set_builder(&then_bb);

                self.vartable.new_scope();

                let reach = self.statement(then)?;

                if reach {
                    unsafe {
                        LLVMBuildBr(self.builder, endif_bb.basic_block);
                    }
                }

                self.add_incoming(&endif_bb);

                self.vartable.leave_scope();

                self.set_builder(&endif_bb);

                return Ok(true);
            },
            Statement::If(cond, then, box Some(else_)) => {
                let mut changeset = HashMap::new();

                then.written_vars(&mut changeset);
                else_.written_vars(&mut changeset);

                let thenbb = self.new_basic_block("then".to_string(), None);
                let elsebb = self.new_basic_block("else".to_string(), None);
                let endifbb = self.new_basic_block("endif".to_string(), Some(&changeset));

                let v = self.expression(cond, ElementaryTypeName::Bool)?;

                unsafe {
                    LLVMBuildCondBr(self.builder, v, thenbb.basic_block, elsebb.basic_block);
                }

                self.set_builder(&thenbb);

                self.vartable.new_scope();

                let reach_then = self.statement(then)?;

                if reach_then {
                    unsafe {
                        LLVMBuildBr(self.builder, endifbb.basic_block);
                    }
                }

                self.add_incoming(&endifbb);

                self.vartable.leave_scope();

                self.set_builder(&elsebb);

                self.vartable.new_scope();

                let reach_else = self.statement(else_)?;

                if reach_else {
                    unsafe {
                        LLVMBuildBr(self.builder, endifbb.basic_block);
                    }
                }

                self.add_incoming(&endifbb);

                self.vartable.leave_scope();

                self.set_builder(&endifbb);

                return Ok(reach_then || reach_else);
            },
            Statement::DoWhile(body, cond) => {
                let mut changeset = HashMap::new();

                body.written_vars(&mut changeset);
                cond.written_vars(&mut changeset);

                let body_bb = self.new_basic_block("body".to_string(), Some(&changeset));

                let end_dowhile_bb  = self.new_basic_block("end_dowhile".to_string(), None);

                self.add_incoming(&body_bb);

                // BODY
                unsafe {
                    LLVMBuildBr(self.builder, body_bb.basic_block);
                }

                self.loop_scope.push(LoopScope{
                    break_bb: end_dowhile_bb,
                    breaks_in_loop: Cell::new(0),
                });

                self.set_builder(&body_bb);

                let reach = self.statement(body)?;

                let scope = self.loop_scope.pop().unwrap();

                let end_dowhile_bb = scope.break_bb;

                if reach {
                    // CONDITION
                    let v = self.expression(cond, ElementaryTypeName::Bool)?;

                    unsafe {
                        LLVMBuildCondBr(self.builder, v, body_bb.basic_block, end_dowhile_bb.basic_block);
                    }
                }

                self.add_incoming(&body_bb);

                self.set_builder(&end_dowhile_bb);

                return Ok(reach || scope.breaks_in_loop.get() > 0);
            },
            Statement::While(cond, body) => {
                let mut changeset = HashMap::new();

                cond.written_vars(&mut changeset);
                body.written_vars(&mut changeset);

                let cond_bb = self.new_basic_block("cond".to_string(), Some(&changeset));

                let body_bb = self.new_basic_block("body".to_string(), None);

                let end_while_bb = self.new_basic_block("end_while".to_string(), None);

                self.add_incoming(&cond_bb);

                // COND
                unsafe {
                    LLVMBuildBr(self.builder, cond_bb.basic_block);
                }
                self.set_builder(&cond_bb);

                self.vartable.new_scope();

                let v = self.expression(cond, ElementaryTypeName::Bool)?;

                unsafe {
                    LLVMBuildCondBr(self.builder, v, body_bb.basic_block, end_while_bb.basic_block);
                }

                self.set_builder(&body_bb);

                self.loop_scope.push(LoopScope{
                    break_bb: end_while_bb,
                    breaks_in_loop: Cell::new(0),
                });

                // BODY
                let reachable = self.statement(body)?;

                let end_while_bb = self.loop_scope.pop().unwrap().break_bb;

                if reachable {
                    unsafe {
                        LLVMBuildBr(self.builder, end_while_bb.basic_block);
                    }
                }

                self.add_incoming(&body_bb);

                self.set_builder(&end_while_bb);

                self.vartable.leave_scope();

                return Ok(true);
            },
            Statement::For(init, box None, next, body) => {
                if let box Some(init) = init {
                    self.statement(init)?;
                }

                let mut changeset = HashMap::new();

                if let box Some(body) = body {
                    body.written_vars(&mut changeset);
                }
                if let box Some(next) = next {
                    next.written_vars(&mut changeset);
                }

                let body_bb = self.new_basic_block("body".to_string(), Some(&changeset));

                let end_for_bb  = self.new_basic_block("end_for".to_string(), None);

                self.add_incoming(&body_bb);
                self.vartable.new_scope();

                unsafe {
                    LLVMBuildBr(self.builder, body_bb.basic_block);
                }

                self.set_builder(&body_bb);

                self.loop_scope.push(LoopScope{
                    break_bb: end_for_bb,
                    breaks_in_loop: Cell::new(0),
                });

                let mut reach = match body {
                    box Some(body) => self.statement(body)?,
                    box None => true
                };

                let scope = self.loop_scope.pop().unwrap();

                let end_for_bb = scope.break_bb;

                if reach {
                    if let box Some(next) = next {
                        // BODY
                        reach = self.statement(next)?;
                    }
                }

                if reach {
                    unsafe {
                        LLVMBuildBr(self.builder, body_bb.basic_block);
                    }
                }

                self.add_incoming(&body_bb);
                self.vartable.leave_scope();
                self.set_builder(&end_for_bb);

                return Ok(scope.breaks_in_loop.get() > 0);
            },
            Statement::For(init, box Some(cond), next, body) => {
                if let box Some(init) = init {
                    self.statement(init)?;
                }

                let mut changeset = HashMap::new();

                cond.written_vars(&mut changeset);

                if let box Some(body) = body {
                    body.written_vars(&mut changeset);
                }
                if let box Some(next) = next {
                    next.written_vars(&mut changeset);
                }

                let cond_bb = self.new_basic_block("cond".to_string(), Some(&changeset));

                let body_bb = self.new_basic_block("body".to_string(), None);

                let end_for_bb = self.new_basic_block("end_for".to_string(), Some(&changeset));

                self.add_incoming(&cond_bb);

                // COND
                unsafe {
                    LLVMBuildBr(self.builder, cond_bb.basic_block);
                }

                self.set_builder(&cond_bb);

                self.vartable.new_scope();

                let v = self.expression(cond, ElementaryTypeName::Bool)?;

                unsafe {
                    LLVMBuildCondBr(self.builder, v, body_bb.basic_block, end_for_bb.basic_block);
                }

                self.add_incoming(&end_for_bb);

                self.set_builder(&body_bb);

                self.loop_scope.push(LoopScope{
                    break_bb: end_for_bb,
                    breaks_in_loop: Cell::new(0),
                });

                let mut reach = match body {
                    box Some(body) => self.statement(body)?,
                    box None => false
                };

                let end_for_bb = self.loop_scope.pop().unwrap().break_bb;

                if reach {
                    if let box Some(next) = next {
                        // BODY
                        reach = self.statement(next)?;
                    }
                }

                self.add_incoming(&cond_bb);

                self.vartable.leave_scope();

                if reach {
                    unsafe {
                        LLVMBuildBr(self.builder, cond_bb.basic_block);
                    }
                }

                self.set_builder(&end_for_bb);

                return Ok(reach);
            },
            Statement::Break => {
                let len = self.loop_scope.len();

                if len == 0 {
                    return Err(format!("break statement not in loop"));
                } else {
                    let scope = &self.loop_scope[len - 1];
                    scope.breaks_in_loop.set(scope.breaks_in_loop.get()+1);

                    unsafe {
                        LLVMBuildBr(self.builder, scope.break_bb.basic_block);
                    }

                    self.add_incoming(&scope.break_bb);
                    return Ok(false)
                }
            },
            _ => {
                return Err(format!("statement not implement: {:?}", stmt));
            }
        }

        Ok(true)
    }

    fn expression(&mut self, e: &Expression, t: ElementaryTypeName) -> Result<LLVMValueRef, String> {
        match e {
            Expression::NumberLiteral(n) => {
                let ltype = if t == ElementaryTypeName::Any {
                    unsafe {
                        LLVMIntTypeInContext(self.context, n.bits() as u32)
                    }
                } else {
                    t.LLVMType(self.context)
                };

                let s = n.to_string();

                unsafe {
                    Ok(LLVMConstIntOfStringAndSize(ltype, s.as_ptr() as *const _, s.len() as _, 10))
                }
            },
            Expression::Add(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildAdd(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Subtract(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildSub(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Multiply(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                unsafe {
                    Ok(LLVMBuildMul(self.builder, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Divide(l, r) => {
                let left = self.expression(l, t)?;
                let right = self.expression(r, t)?;

                if t.signed() {
                    unsafe {
                        Ok(LLVMBuildSDiv(self.builder, left, right, b"\0".as_ptr() as *const _))
                    }
                } else {
                    unsafe {
                        Ok(LLVMBuildUDiv(self.builder, left, right, b"\0".as_ptr() as *const _))
                    }
                }
            },
            Expression::Equal(l, r) => {
                let left = self.expression(l, ElementaryTypeName::Uint(32))?;
                let right = self.expression(r, ElementaryTypeName::Uint(32))?;

                unsafe {
                    Ok(LLVMBuildICmp(self.builder, LLVMIntPredicate::LLVMIntEQ, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::More(l, r) => {
                let left = self.expression(l, ElementaryTypeName::Uint(32))?;
                let right = self.expression(r, ElementaryTypeName::Uint(32))?;

                unsafe {
                    Ok(LLVMBuildICmp(self.builder, LLVMIntPredicate::LLVMIntSGT, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Less(l, r) => {
                let left = self.expression(l, ElementaryTypeName::Uint(32))?;
                let right = self.expression(r, ElementaryTypeName::Uint(32))?;

                unsafe {
                    Ok(LLVMBuildICmp(self.builder, LLVMIntPredicate::LLVMIntSLT, left, right, b"\0".as_ptr() as *const _))
                }
            },
            Expression::Variable(_, s) => {
                let var = self.vartable.get(&s.name);

                if var.typ == t || t == ElementaryTypeName::Any {
                    Ok(var.value)
                } else {
                    Ok(match t {
                        ElementaryTypeName::Uint(_) => unsafe {
                            LLVMBuildZExtOrBitCast(self.builder, var.value, t.LLVMType(self.context), "\0".as_ptr() as *const _)
                        },
                        ElementaryTypeName::Int(_) => unsafe {
                            LLVMBuildSExtOrBitCast(self.builder, var.value, t.LLVMType(self.context), "\0".as_ptr() as *const _)
                        },
                        _ => panic!("implement implicit casting for {:?} to {:?}", var.typ, t)
                    })
                }
            },
            Expression::Assign(l, r) => {
                match l {
                    box Expression::Variable(_, s) => {
                        let typ = self.vartable.get_type(&s.name);
                        let value = self.expression(r, typ)?;
                        self.vartable.set_value(&s.name, value);
                        Ok(0 as LLVMValueRef)
                    },
                    _ => panic!("cannot assign to non-lvalue")
                }
            },
            Expression::AssignAdd(l, r) => {
                match l {
                    box Expression::Variable(_, s) => {
                        let typ = self.vartable.get_type(&s.name);
                        let value = self.expression(r, typ)?;
                        let lvalue = self.vartable.get_value(&s.name);
                        self.vartable.set_value(&s.name, value);
                        let nvalue = unsafe {
                            LLVMBuildAdd(self.builder, lvalue, value, b"\0".as_ptr() as *const _)
                        };
                        self.vartable.set_value(&s.name, nvalue);
                        Ok(0 as LLVMValueRef)
                    },
                    _ => panic!("cannot assign to non-lvalue")
                }
            },
            Expression::AssignSubtract(l, r) => {
                match l {
                    box Expression::Variable(_, s) => {
                        let typ = self.vartable.get_type(&s.name);
                        let value = self.expression(r, typ)?;
                        let lvalue = self.vartable.get_value(&s.name);
                        self.vartable.set_value(&s.name, value);
                        let nvalue = unsafe {
                            LLVMBuildSub(self.builder, lvalue, value, b"\0".as_ptr() as *const _)
                        };
                        self.vartable.set_value(&s.name, nvalue);
                        Ok(0 as LLVMValueRef)
                    },
                    _ => panic!("cannot assign to non-lvalue")
                }
            },
            Expression::AssignMultiply(l, r) => {
                match l {
                    box Expression::Variable(_, s) => {
                        let typ = self.vartable.get_type(&s.name);
                        let value = self.expression(r, typ)?;
                        let lvalue = self.vartable.get_value(&s.name);
                        self.vartable.set_value(&s.name, value);
                        let nvalue = unsafe {
                            LLVMBuildMul(self.builder, lvalue, value, b"\0".as_ptr() as *const _)
                        };
                        self.vartable.set_value(&s.name, nvalue);
                        Ok(0 as LLVMValueRef)
                    },
                    _ => panic!("cannot assign to non-lvalue")
                }
            },
            Expression::PostDecrement(box Expression::Variable(t, s)) => {
                let before_value = self.vartable.get_value(&s.name);
                let after_value = unsafe {
                    LLVMBuildSub(self.builder, before_value, LLVMConstInt(t.get().LLVMType(self.context), 1, LLVM_FALSE), b"\0".as_ptr() as *const _)
                };

                self.vartable.set_value(&s.name, after_value);

                Ok(before_value)
            },
            Expression::PreDecrement(box Expression::Variable(t, s)) => {
                let before_value = self.vartable.get_value(&s.name);
                let after_value = unsafe {
                    LLVMBuildSub(self.builder, before_value, LLVMConstInt(t.get().LLVMType(self.context), 1, LLVM_FALSE), b"\0".as_ptr() as *const _)
                };

                self.vartable.set_value(&s.name, after_value);

                Ok(after_value)
            },
            Expression::PostIncrement(box Expression::Variable(t, s)) => {
                let before_value = self.vartable.get_value(&s.name);
                let after_value = unsafe {
                    LLVMBuildAdd(self.builder, before_value, LLVMConstInt(t.get().LLVMType(self.context), 1, LLVM_FALSE), b"\0".as_ptr() as *const _)
                };

                self.vartable.set_value(&s.name, after_value);

                Ok(before_value)
            },
            Expression::PreIncrement(box Expression::Variable(t, s)) => {
                let before_value = self.vartable.get_value(&s.name);
                let after_value = unsafe {
                        LLVMBuildAdd(self.builder, before_value, LLVMConstInt(t.get().LLVMType(self.context), 1, LLVM_FALSE), b"\0".as_ptr() as *const _)
                };

                self.vartable.set_value(&s.name, after_value);

                Ok(after_value)
            },
            _ => {
                Err(format!("expression not implemented: {:?}", e))
            }
        }
    }

    fn new_basic_block(&self, name: String, phi: Option<&HashMap<String, ElementaryTypeName>>) -> BasicBlock {
        let cstr = CString::new(name).unwrap();

        let mut bb = BasicBlock{
            basic_block: unsafe { LLVMAppendBasicBlockInContext(self.context, self.llfunction, cstr.as_ptr() as *const _) },
            phi: HashMap::new(),
        };

        if let Some(names) = phi {
            unsafe {
                LLVMPositionBuilderAtEnd(self.builder, bb.basic_block);
            }

            for (name, ty) in names {
                if bb.phi.contains_key(name) {
                    continue;
                }

                let cname = CString::new(name.to_string()).unwrap();
                let phi = unsafe {
                    LLVMBuildPhi(self.builder, ty.LLVMType(self.context), cname.as_ptr() as *const _)
                };

                bb.phi.insert(name.to_string(), phi);
            }

            unsafe {
                LLVMPositionBuilderAtEnd(self.builder, self.basic_block);
            }
        }

        bb
    }

    fn add_incoming(&self, bb: &BasicBlock) {
        for (name, phi) in &bb.phi {
            let mut values = vec!(self.vartable.get_value(name));
            let mut blocks = vec!(self.basic_block);

            unsafe {
                LLVMAddIncoming(*phi, values.as_mut_ptr(), blocks.as_mut_ptr(), 1);
            }
        }
    }

    fn set_builder(&mut self, bb: &BasicBlock) {
        unsafe {
            LLVMPositionBuilderAtEnd(self.builder, bb.basic_block);
        }

        self.basic_block = bb.basic_block;

        for (name, phi) in &bb.phi {
            self.vartable.set_value(name, *phi);
        }
    }
}
