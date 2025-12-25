/// Immediately invokes a closure with optional parameters.
#[macro_export]
macro_rules! invoke {
    (move $($param:ident),+ => $body:expr) => {
        (move |$($param),*| $body)($($param),*)
    };
    (move $body:expr) => {
        (move || $body)()
    };
    ($($param:ident),+ => $body:expr) => {
        (|$($param),*| $body)($($param),*)
    };
    ($body:expr) => {
        (|| $body)()
    };
}

#[cfg(test)]
mod tests {
    extern crate alloc;
    use alloc::string::String;

    #[test]
    fn test_invoke_with_params() {
        let a = 5;
        let b = 6;
        let result = invoke!(a, b => { a + b });
        assert_eq!(result, 11);
    }

    #[test]
    fn test_invoke_no_params() {
        let result = invoke!({ 42 });
        assert_eq!(result, 42);
    }

    #[test]
    fn test_invoke_move_no_params() {
        let s = String::from("test");
        let result = invoke!(move { s.len() });
        assert_eq!(result, 4);
    }

    #[test]
    fn test_invoke_move_with_params() {
        let a = 10;
        let b = 20;
        let result = invoke!(move a, b => { a * b });
        assert_eq!(result, 200);
    }

    #[test]
    fn test_invoke_with_mutation() {
        let mut x = 0;
        invoke!({ x = 42 });
        assert_eq!(x, 42);
    }

    #[test]
    fn test_invoke_multiple_params() {
        let a = 1;
        let b = 2;
        let c = 3;
        let result = invoke!(a, b, c => { a + b + c });
        assert_eq!(result, 6);
    }
}
