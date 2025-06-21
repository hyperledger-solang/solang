// SPDX-License-Identifier: Apache-2.0

use solang::codegen::Options;
use solang::file_resolver::FileResolver;
use solang::sema::diagnostics::Diagnostics;
use solang::{compile, Target};
use std::ffi::OsStr;

fn build_with_strict_soroban_types(src: &str, strict: bool) -> Diagnostics {
    let tmp_file = OsStr::new("test.sol");
    let mut cache = FileResolver::default();
    cache.set_file_contents(tmp_file.to_str().unwrap(), src.to_string());
    let opt = inkwell::OptimizationLevel::Default;
    let target = Target::Soroban;
    let (_, ns) = compile(
        tmp_file,
        &mut cache,
        target,
        &Options {
            opt_level: opt.into(),
            log_runtime_errors: true,
            log_prints: true,
            strict_soroban_types: strict,
            #[cfg(feature = "wasm_opt")]
            wasm_opt: Some(contract_build::OptimizationPasses::Z),
            soroban_version: None,
            ..Default::default()
        },
        std::vec!["unknown".to_string()],
        "0.0.1",
    );
    ns.diagnostics
}

#[test]
fn test_warning_for_int56_without_strict() {
    let src = r#"contract test {
        function test_int56(int56 a) public returns (int64) {
            return int64(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about int56 being rounded to int64
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for int56 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("int56") && msg.contains("int64")), 
            "Expected warning about int56 being rounded to int64");
}

#[test]
fn test_warning_for_uint56_without_strict() {
    let src = r#"contract test {
        function test_uint56(uint56 a) public returns (uint64) {
            return uint64(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about uint56 being rounded to uint64
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for uint56 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("uint56") && msg.contains("uint64")), 
            "Expected warning about uint56 being rounded to uint64");
}

#[test]
fn test_warning_for_int96_without_strict() {
    let src = r#"contract test {
        function test_int96(int96 a) public returns (int128) {
            return int128(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about int96 being rounded to int128
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for int96 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("int96") && msg.contains("int128")), 
            "Expected warning about int96 being rounded to int128");
}

#[test]
fn test_warning_for_uint96_without_strict() {
    let src = r#"contract test {
        function test_uint96(uint96 a) public returns (uint128) {
            return uint128(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about uint96 being rounded to uint128
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for uint96 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("uint96") && msg.contains("uint128")), 
            "Expected warning about uint96 being rounded to uint128");
}

#[test]
fn test_warning_for_int200_without_strict() {
    let src = r#"contract test {
        function test_int200(int200 a) public returns (int256) {
            return int256(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about int200 being rounded to int256
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for int200 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("int200") && msg.contains("int256")), 
            "Expected warning about int200 being rounded to int256");
}

#[test]
fn test_warning_for_uint200_without_strict() {
    let src = r#"contract test {
        function test_uint200(uint200 a) public returns (uint256) {
            return uint256(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have a warning about uint200 being rounded to uint256
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for uint200 rounding");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    assert!(warning_messages.iter().any(|msg| msg.contains("uint200") && msg.contains("uint256")), 
            "Expected warning about uint200 being rounded to uint256");
}

#[test]
fn test_error_for_int56_with_strict() {
    let src = r#"contract test {
        function test_int56(int56 a) public returns (int64) {
            return int64(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about int56 being rounded to int64
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for int56 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("int56") && msg.contains("int64")), 
            "Expected error about int56 being rounded to int64");
}

#[test]
fn test_error_for_uint56_with_strict() {
    let src = r#"contract test {
        function test_uint56(uint56 a) public returns (uint64) {
            return uint64(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about uint56 being rounded to uint64
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for uint56 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("uint56") && msg.contains("uint64")), 
            "Expected error about uint56 being rounded to uint64");
}

#[test]
fn test_error_for_int96_with_strict() {
    let src = r#"contract test {
        function test_int96(int96 a) public returns (int128) {
            return int128(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about int96 being rounded to int128
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for int96 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("int96") && msg.contains("int128")), 
            "Expected error about int96 being rounded to int128");
}

#[test]
fn test_error_for_uint96_with_strict() {
    let src = r#"contract test {
        function test_uint96(uint96 a) public returns (uint128) {
            return uint128(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about uint96 being rounded to uint128
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for uint96 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("uint96") && msg.contains("uint128")), 
            "Expected error about uint96 being rounded to uint128");
}

#[test]
fn test_error_for_int200_with_strict() {
    let src = r#"contract test {
        function test_int200(int200 a) public returns (int256) {
            return int256(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about int200 being rounded to int256
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for int200 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("int200") && msg.contains("int256")), 
            "Expected error about int200 being rounded to int256");
}

#[test]
fn test_error_for_uint200_with_strict() {
    let src = r#"contract test {
        function test_uint200(uint200 a) public returns (uint256) {
            return uint256(a);
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, true);
    
    // Should have an error about uint200 being rounded to uint256
    let errors: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Error).collect();
    assert!(!errors.is_empty(), "Expected errors for uint200 rounding with strict mode");
    
    let error_messages: Vec<_> = errors.iter().map(|e| e.message.as_str()).collect();
    assert!(error_messages.iter().any(|msg| msg.contains("uint200") && msg.contains("uint256")), 
            "Expected error about uint200 being rounded to uint256");
}

#[test]
fn test_no_warnings_for_soroban_compatible_types() {
    let src = r#"contract test {
        function test_int32(int32 a) public returns (int32) {
            return a;
        }
        
        function test_uint32(uint32 a) public returns (uint32) {
            return a;
        }
        
        function test_int64(int64 a) public returns (int64) {
            return a;
        }
        
        function test_uint64(uint64 a) public returns (uint64) {
            return a;
        }
        
        function test_int128(int128 a) public returns (int128) {
            return a;
        }
        
        function test_uint128(uint128 a) public returns (uint128) {
            return a;
        }
        
        function test_int256(int256 a) public returns (int256) {
            return a;
        }
        
        function test_uint256(uint256 a) public returns (uint256) {
            return a;
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have no warnings for Soroban-compatible types
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    
    // Check that no warnings are related to integer width rounding
    assert!(!warning_messages.iter().any(|msg| msg.contains("is not supported by the Soroban runtime")), 
            "Should not have warnings for Soroban-compatible integer types");
}

#[test]
fn test_multiple_rounding_warnings() {
    let src = r#"contract test {
        function test_multiple(int56 a, uint56 b, int96 c, uint96 d) 
            public returns (int64, uint64, int128, uint128) {
            return (int64(a), uint64(b), int128(c), uint128(d));
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have warnings for all non-Soroban-compatible types
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(warnings.len() >= 4, "Expected at least 4 warnings for different integer types");
    
    let warning_messages: Vec<_> = warnings.iter().map(|w| w.message.as_str()).collect();
    
    // Check for specific type warnings
    assert!(warning_messages.iter().any(|msg| msg.contains("int56") && msg.contains("int64")));
    assert!(warning_messages.iter().any(|msg| msg.contains("uint56") && msg.contains("uint64")));
    assert!(warning_messages.iter().any(|msg| msg.contains("int96") && msg.contains("int128")));
    assert!(warning_messages.iter().any(|msg| msg.contains("uint96") && msg.contains("uint128")));
}

#[test]
fn test_variable_declaration_warnings() {
    let src = r#"contract test {
        function test_variables() public returns (int64, uint64, int128, uint128) {
            int56 a = 42;
            uint56 b = 43;
            int96 c = 44;
            uint96 d = 45;
            
            return (int64(a), uint64(b), int128(c), uint128(d));
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have warnings for variable declarations
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for variable declarations with non-Soroban-compatible types");
}

#[test]
fn test_struct_field_warnings() {
    let src = r#"contract test {
        struct Data {
            int56 a;
            uint56 b;
            int96 c;
            uint96 d;
        }
        
        function test_struct() public returns (int64, uint64, int128, uint128) {
            Data memory data = Data(42, 43, 44, 45);
            return (int64(data.a), uint64(data.b), int128(data.c), uint128(data.d));
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have warnings for struct field declarations
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for struct fields with non-Soroban-compatible types");
}

#[test]
fn test_array_type_warnings() {
    let src = r#"contract test {
        function test_arrays() public returns (int64, uint64, int128, uint128) {
            int56[] memory int_array = new int56[](1);
            uint56[] memory uint_array = new uint56[](1);
            int96[] memory int96_array = new int96[](1);
            uint96[] memory uint96_array = new uint96[](1);
            
            int_array[0] = 42;
            uint_array[0] = 43;
            int96_array[0] = 44;
            uint96_array[0] = 45;
            
            return (int64(int_array[0]), uint64(uint_array[0]), 
                    int128(int96_array[0]), uint128(uint96_array[0]));
        }
    }"#;
    
    let diagnostics = build_with_strict_soroban_types(src, false);
    
    // Should have warnings for array type declarations
    let warnings: Vec<_> = diagnostics.iter().filter(|d| d.level == solang_parser::diagnostics::Level::Warning).collect();
    assert!(!warnings.is_empty(), "Expected warnings for array types with non-Soroban-compatible element types");
} 