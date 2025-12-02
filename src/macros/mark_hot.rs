/// Marks a function as hot path for verification.
pub macro mark_hot($func:ident) {
    $crate::paste::paste! {
        #[used]
        #[unsafe(link_section = ".hot_funcs")]
        static [<HOT_FUNC_ $func:upper>]: &str = concat!(module_path!(), "::", stringify!($func), "\0");
    }
}

#[cfg(test)]
mod tests {
    use super::mark_hot;

    #[test]
    fn test_mark_hot_compiles() {
        fn test_fn() {
            mark_hot!(test_fn);
        }
        test_fn();
    }

    #[test]
    fn test_mark_hot_with_return() {
        fn add(a: i32, b: i32) -> i32 {
            mark_hot!(add);
            a + b
        }
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn test_multiple_hot_functions() {
        fn func1() -> i32 {
            mark_hot!(func1);
            42
        }
        fn func2() -> i32 {
            mark_hot!(func2);
            100
        }
        assert_eq!(func1(), 42);
        assert_eq!(func2(), 100);
    }
}
