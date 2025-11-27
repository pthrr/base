macro_rules! invoke {
    // No parameters
    (move $body:expr) => {
        (move || $body)()
    };
    ($body:expr) => {
        (|| $body)()
    };
    // With parameters
    (move $($param:ident),+ => $body:expr) => {
        (move |$($param),*| $body)($($param),*)
    };
    ($($param:ident),+ => $body:expr) => {
        (|$($param),*| $body)($($param),*)
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_invoke_param() {
        let a = 5;
        let b = 6;
        let result = invoke!(a, b => {
            a + b
        });
        assert_eq!(result, 11);
    }
}
