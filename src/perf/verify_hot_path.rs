//! Hot path verification via LLVM IR analysis.
//!
//! Verifies functions marked with `mark_hot!` by analyzing LLVM IR for
//! real-time safety violations and performance issues.
//!
//! Use `HotPathVerifier` with custom checks or `verify_hot_function()` for defaults.

use std::collections::HashSet;

/// Check severity: Error (hard fail) or Warning (performance note).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Severity {
    Error,
    Warning,
}

/// Trait for hot path verification checks.
pub trait HotPathCheck: Send + Sync {
    fn name(&self) -> &str;
    fn severity(&self) -> Severity;
    fn check_line(&self, line: &str) -> Option<String>;
}

/// Check for memory allocations.
pub struct AllocationCheck;
impl HotPathCheck for AllocationCheck {
    fn name(&self) -> &str {
        "allocation"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("call")
            && (line.contains("@malloc")
                || line.contains("@calloc")
                || line.contains("@realloc")
                || line.contains("@alloc")
                || line.contains("@__rust_alloc")
                || line.contains("@__rust_realloc"))
        {
            Some("contains allocation (real-time violation)".to_string())
        } else {
            None
        }
    }
}

/// Check for atomic operations.
pub struct AtomicCheck;
impl HotPathCheck for AtomicCheck {
    fn name(&self) -> &str {
        "atomic"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("atomicrmw") || line.contains("cmpxchg") || line.contains(" fence ") {
            Some("contains atomic operation (real-time violation)".to_string())
        } else {
            None
        }
    }
}

/// Check for indirect control flow.
pub struct IndirectionCheck;
impl HotPathCheck for IndirectionCheck {
    fn name(&self) -> &str {
        "indirection"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("invoke") || line.contains("callbr") {
            Some("contains indirection".to_string())
        } else {
            None
        }
    }
}

/// Check for non-inlined function calls.
pub struct FunctionCallCheck;
impl HotPathCheck for FunctionCallCheck {
    fn name(&self) -> &str {
        "function_call"
    }
    fn severity(&self) -> Severity {
        Severity::Error
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("call") && !line.contains("@llvm.") {
            // Skip if it's an allocation (handled by AllocationCheck)
            if line.contains("@malloc")
                || line.contains("@calloc")
                || line.contains("@realloc")
                || line.contains("@alloc")
            {
                return None;
            }
            Some("contains function call (not inlined)".to_string())
        } else {
            None
        }
    }
}

/// Check for volatile loads.
pub struct VolatileLoadCheck;
impl HotPathCheck for VolatileLoadCheck {
    fn name(&self) -> &str {
        "volatile_load"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("load") && line.contains("volatile") {
            Some("volatile load (forces memory access, ~100-300 cycles)".to_string())
        } else {
            None
        }
    }
}

/// Check for volatile stores.
pub struct VolatileStoreCheck;
impl HotPathCheck for VolatileStoreCheck {
    fn name(&self) -> &str {
        "volatile_store"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("store") && line.contains("volatile") {
            Some("volatile store (forces write-through, ~100-300 cycles)".to_string())
        } else {
            None
        }
    }
}

/// Check for division/modulo operations.
pub struct DivisionCheck;
impl HotPathCheck for DivisionCheck {
    fn name(&self) -> &str {
        "division"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains(" sdiv ")
            || line.contains(" udiv ")
            || line.contains(" srem ")
            || line.contains(" urem ")
        {
            Some("division/modulo operation (10-40 cycles, not pipelined)".to_string())
        } else {
            None
        }
    }
}

/// Check for unaligned memory access.
pub struct UnalignedAccessCheck;
impl HotPathCheck for UnalignedAccessCheck {
    fn name(&self) -> &str {
        "unaligned_access"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if (line.contains("load") || line.contains("store")) && line.contains("align 1") {
            Some("unaligned memory access (2-10x slower, blocks SIMD)".to_string())
        } else {
            None
        }
    }
}

/// Check for non-inbounds GEP.
pub struct NonInboundsGepCheck;
impl HotPathCheck for NonInboundsGepCheck {
    fn name(&self) -> &str {
        "non_inbounds_gep"
    }
    fn severity(&self) -> Severity {
        Severity::Warning
    }
    fn check_line(&self, line: &str) -> Option<String> {
        if line.contains("getelementptr") && !line.contains("inbounds") {
            Some("non-inbounds GEP (adds bounds checks, prevents optimization)".to_string())
        } else {
            None
        }
    }
}

/// Verifier for hot path functions with configurable checks.
pub struct HotPathVerifier {
    checks: Vec<Box<dyn HotPathCheck>>,
}

impl HotPathVerifier {
    pub fn new() -> Self {
        Self { checks: Vec::new() }
    }

    pub fn with_check(mut self, check: Box<dyn HotPathCheck>) -> Self {
        self.checks.push(check);
        self
    }

    pub fn with_default_checks(self) -> Self {
        self.with_check(Box::new(IndirectionCheck))
            .with_check(Box::new(AllocationCheck))
            .with_check(Box::new(FunctionCallCheck))
            .with_check(Box::new(AtomicCheck))
            .with_check(Box::new(VolatileLoadCheck))
            .with_check(Box::new(VolatileStoreCheck))
            .with_check(Box::new(DivisionCheck))
            .with_check(Box::new(UnalignedAccessCheck))
            .with_check(Box::new(NonInboundsGepCheck))
    }

    pub fn verify(&self, ir: &str, func_name: &str) -> Result<Vec<String>, String> {
        let body = find_function_body(ir, func_name)?;
        let mut warnings = Vec::new();

        for line in body.lines() {
            for check in &self.checks {
                if let Some(violation) = check.check_line(line) {
                    match check.severity() {
                        Severity::Error => {
                            return Err(format!("{}: {}", func_name, violation));
                        }
                        Severity::Warning => {
                            warnings.push(format!("{}: {}", func_name, violation));
                        }
                    }
                }
            }
        }

        Ok(warnings)
    }
}

impl Default for HotPathVerifier {
    fn default() -> Self {
        Self::new().with_default_checks()
    }
}

/// Verifies hot path functions from LLVM IR content using default checks.
pub fn verify_hot_path_functions(ir: &str) -> Result<(), Box<dyn std::error::Error>> {
    let verifier = HotPathVerifier::default();
    let hot_funcs = find_hot_functions_from_ir(ir);

    for func in hot_funcs {
        verifier.verify(ir, &func).map_err(|e| e)?;
    }

    Ok(())
}

/// Discovers hot functions from LLVM IR `.hot_funcs` section.
pub fn find_hot_functions_from_ir(ir: &str) -> HashSet<String> {
    use regex::Regex;
    let mut hot_funcs = HashSet::new();

    // Find .hot_funcs entries with ptr @alloc_*
    let re_ptr = Regex::new(r#"ptr\s+(@alloc_\w+).*section\s+"\.hot_funcs""#).unwrap();

    for cap in re_ptr.captures_iter(ir) {
        if let Some(alloc_ref) = cap.get(1) {
            // Find allocation: @alloc_HASH = ... c"function_name\00"
            let alloc_pattern = format!(
                r#"{}\s*=.*?c"([^"]+)\\00""#,
                regex::escape(alloc_ref.as_str())
            );
            if let Ok(re_alloc) = Regex::new(&alloc_pattern) {
                if let Some(alloc_cap) = re_alloc.captures(ir) {
                    if let Some(func_name) = alloc_cap.get(1) {
                        hot_funcs.insert(func_name.as_str().to_string());
                    }
                }
            }
        }
    }

    hot_funcs
}

/// Converts Rust path (a::b::c) to LLVM mangled format (1a1b1c).
fn mangle_rust_path(path: &str) -> String {
    path.split("::")
        .map(|segment| format!("{}{}", segment.len(), segment))
        .collect::<Vec<_>>()
        .join("")
}

/// Extracts function body from LLVM IR.
fn find_function_body(ir: &str, func_name: &str) -> Result<String, String> {
    use regex::Regex;

    // Mangle Rust paths (a::b::c) for matching in IR
    let search_name = if func_name.contains("::") {
        mangle_rust_path(func_name)
    } else {
        func_name.to_string()
    };

    let pattern = format!(
        r"define[^@]*@[^\s]*{}[^\(]*\([^\)]*\)[^\{{]*\{{(.*?)\n\}}",
        regex::escape(&search_name)
    );
    let re = Regex::new(&pattern).unwrap();

    let body = re
        .captures(ir)
        .ok_or_else(|| format!("Function {} not found in IR", func_name))?
        .get(1)
        .unwrap()
        .as_str()
        .to_string();

    Ok(body)
}

/// Verifies a single hot function using default checks.
pub fn verify_hot_function(ir: &str, func_name: &str) -> Result<(), String> {
    let verifier = HotPathVerifier::default();
    verifier.verify(ir, func_name).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_find_mark_hot_in_ir() {
        let test_ir = r#"
            @alloc_foo = private unnamed_addr constant [4 x i8] c"foo\00", align 1
            @HOT_FUNC.1 = internal constant <{ ptr, [8 x i8] }> <{ ptr @alloc_foo, [8 x i8] c"\03\00\00\00\00\00\00\00" }>, section ".hot_funcs", align 8
            @alloc_bar = private unnamed_addr constant [4 x i8] c"bar\00", align 1
            @HOT_FUNC.2 = internal constant <{ ptr, [8 x i8] }> <{ ptr @alloc_bar, [8 x i8] c"\03\00\00\00\00\00\00\00" }>, section ".hot_funcs", align 8

            define i32 @foo() { ret i32 42 }
            define i32 @bar() { ret i32 24 }
            define i32 @baz() { ret i32 10 }
        "#;

        let found = find_hot_functions_from_ir(test_ir);

        assert_eq!(found.len(), 2);
        assert!(found.contains("foo"));
        assert!(found.contains("bar"));
        assert!(!found.contains("baz"));
    }

    #[test]
    fn test_detect_allocation() {
        let ir = "define i32 @test_func() {  %1 = call ptr @malloc(i64 16)  ret i32 0\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("allocation"));
    }

    #[test]
    fn test_detect_function_call() {
        let ir = "define i32 @test_func() {  %1 = call i32 @other_function()  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("function call"));
    }

    #[test]
    fn test_detect_atomic_operation() {
        let ir = "define i32 @test_func(ptr %ptr) {  %1 = atomicrmw add ptr %ptr, i32 1 seq_cst  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("atomic"));
    }

    #[test]
    fn test_detect_indirection() {
        let ir = "define i32 @test_func() {  %1 = invoke i32 @foo() to label %normal unwind label %error\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("indirection"));
    }

    #[test]
    fn test_allow_llvm_intrinsics() {
        let ir = "define i32 @test_func(i32 %a, i32 %b) {  %1 = call i32 @llvm.sadd.sat.i32(i32 %a, i32 %b)  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pure_arithmetic() {
        let ir = "define i32 @test_func(i32 %a, i32 %b) {  %1 = add i32 %a, %b  %2 = mul i32 %1, 2  ret i32 %2\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pure_with_branches() {
        let ir = "define i32 @test_func(i32 %a) {  %1 = icmp sgt i32 %a, 0  br i1 %1, label %positive, label %negative\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_pure_with_loads_stores() {
        let ir = "define i32 @test_func(ptr %ptr) {  %1 = load i32, ptr %ptr, align 4  %2 = add i32 %1, 1  store i32 %2, ptr %ptr, align 4  ret i32 %2\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_detect_fence() {
        let ir = "define void @test_func() {  fence seq_cst  ret void\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("atomic"));
    }

    #[test]
    fn test_detect_cmpxchg() {
        let ir = "define i32 @test_func(ptr %ptr) {  %1 = cmpxchg ptr %ptr, i32 0, i32 1 seq_cst seq_cst  ret i32 0\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("atomic"));
    }

    #[test]
    fn test_detect_callbr() {
        let ir = "define i32 @test_func() {  callbr i32 0 to label %normal  ret i32 0\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("indirection"));
    }

    #[test]
    fn test_warn_division() {
        let ir = "define i32 @test_func(i32 %a, i32 %b) {  %1 = sdiv i32 %a, %b  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_modulo() {
        let ir = "define i32 @test_func(i32 %a, i32 %b) {  %1 = urem i32 %a, %b  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_volatile_load() {
        let ir =
            "define i32 @test_func(ptr %ptr) {  %1 = load volatile i32, ptr %ptr  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_volatile_store() {
        let ir = "define void @test_func(ptr %ptr, i32 %val) {  store volatile i32 %val, ptr %ptr  ret void\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_unaligned_access() {
        let ir =
            "define i32 @test_func(ptr %ptr) {  %1 = load i32, ptr %ptr, align 1  ret i32 %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_warn_non_inbounds_gep() {
        let ir = "define ptr @test_func(ptr %ptr) {  %1 = getelementptr i32, ptr %ptr, i32 1  ret ptr %1\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    #[test]
    fn test_function_not_found() {
        let ir = "define i32 @other_func() { ret i32 0\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("not found"));
    }

    #[test]
    fn test_multiple_violations() {
        let ir = "define i32 @test_func() {  %1 = call ptr @malloc(i64 16)  %2 = atomicrmw add ptr %1, i32 1 seq_cst  ret i32 0\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("allocation"));
    }

    #[test]
    fn test_empty_function() {
        let ir = "define void @test_func() {\n}";
        let result = verify_hot_function(ir, "test_func");
        assert!(result.is_ok());
    }

    // Common real-world use cases

    #[test]
    fn test_simple_math_hot_path() {
        // Common case: simple arithmetic that should be fully optimized
        let ir = "define i32 @calculate(i32 %a, i32 %b) {  %1 = add i32 %a, %b  %2 = mul i32 %1, 2  ret i32 %2\n}";
        let result = verify_hot_function(ir, "calculate");
        assert!(result.is_ok());
    }

    #[test]
    fn test_array_sum_hot_path() {
        // Common case: loop with loads and simple operations
        let ir = "define i32 @sum_array(ptr %arr, i32 %len) {  %1 = load i32, ptr %arr, align 4  ret i32 %1\n}";
        let result = verify_hot_function(ir, "sum_array");
        assert!(result.is_ok());
    }

    #[test]
    fn test_conditional_hot_path() {
        // Common case: branches and conditionals
        let ir = "define i32 @max(i32 %a, i32 %b) {  %1 = icmp sgt i32 %a, %b  br i1 %1, label %true, label %false  ret i32 %a\n}";
        let result = verify_hot_function(ir, "max");
        assert!(result.is_ok());
    }

    #[test]
    fn test_find_hot_functions_empty_ir() {
        let ir = "define i32 @foo() { ret i32 0 }";
        let funcs = find_hot_functions_from_ir(ir);
        assert!(funcs.is_empty());
    }

    #[test]
    fn test_find_hot_functions_single() {
        let ir = r#"
            @alloc_process = private unnamed_addr constant [8 x i8] c"process\00", align 1
            @HOT_FUNC = constant <{ ptr, [8 x i8] }> <{ ptr @alloc_process, [8 x i8] c"\07\00\00\00\00\00\00\00" }>, section ".hot_funcs", align 8
        "#;
        let funcs = find_hot_functions_from_ir(ir);
        assert_eq!(funcs.len(), 1);
        assert!(funcs.contains("process"));
    }

    #[test]
    fn test_mangle_rust_path() {
        assert_eq!(mangle_rust_path("foo"), "3foo");
        assert_eq!(mangle_rust_path("foo::bar"), "3foo3bar");
        assert_eq!(
            mangle_rust_path("tinywdf::dag::node_arena::get_children_of"),
            "7tinywdf3dag10node_arena15get_children_of"
        );
    }
}
