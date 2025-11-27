#[macro_export]
macro_rules! invoke {
    (move $($param:ident),+ => $body:expr) => {
        (move |$($param),*| $body)($($param),*)
    };
    ($($param:ident),+ => $body:expr) => {
        (|$($param),*| $body)($($param),*)
    };
    (move $body:expr) => {
        (move || $body)()
    };
    ($body:expr) => {
        (|| $body)()
    };
}

#[cfg(test)]
#[allow(clippy::redundant_closure_call)]
mod tests {
    extern crate alloc;
    use alloc::string::String;

    #[test]
    fn invokes_with_params() {
        let a = 5;
        let b = 6;
        assert_eq!(invoke!(a, b => a + b), 11);
    }

    #[test]
    fn invokes_without_params() {
        assert_eq!(invoke!({ 42 }), 42);
    }

    #[test]
    fn invokes_move_without_params() {
        let s = String::from("test");
        assert_eq!(invoke!(move { s.len() }), 4);
    }

    #[test]
    fn invokes_move_with_params() {
        let a = String::from("x");
        let b = String::from("y");
        assert_eq!(
            invoke!(move a, b => <String>::len(&a) + <String>::len(&b)),
            2
        );
    }
}
