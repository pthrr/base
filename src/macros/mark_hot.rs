#[macro_export]
macro_rules! mark_hot {
    ($function:path) => {
        const _: () = {
            #[repr(transparent)]
            struct HotFunction(*const ());

            // SAFETY: HotFunction wraps a raw function pointer treated as opaque
            // build-time metadata. The static below lives in the `.hot_funcs`
            // linker section so external IR-scanning tooling can enumerate hot
            // symbols; the runtime never dereferences the pointer or observes
            // the address across threads, so `Sync` is trivially sound.
            unsafe impl Sync for HotFunction {}

            // `#[inline(never)]` anchor + `#[used]` reference keep `$function`
            // from being collapsed by LTO. Without an outlined symbol the IR
            // verifier cannot match the marker against a `define` line.
            #[inline(never)]
            fn __hot_anchor() -> *const () {
                $function as *const ()
            }

            #[used]
            static __HOT_ANCHOR_REF: HotFunction = HotFunction(__hot_anchor as *const ());

            #[used]
            #[link_section = ".hot_funcs"]
            static HOT_FUNCTION: HotFunction = HotFunction($function as *const ());
        };
    };
}

#[cfg(test)]
mod tests {
    #[test]
    fn marks_function() {
        fn add(a: i32, b: i32) -> i32 {
            mark_hot!(add);
            a + b
        }
        assert_eq!(add(2, 3), 5);
    }

    #[test]
    fn marks_associated_function() {
        struct Operations;

        impl Operations {
            fn identity(value: i32) -> i32 {
                mark_hot!(Operations::identity);
                value
            }
        }

        assert_eq!(Operations::identity(42), 42);
    }

    #[test]
    fn supports_multiple_markers_in_one_scope() {
        fn identity(value: i32) -> i32 {
            mark_hot!(identity);
            mark_hot!(identity);
            value
        }

        assert_eq!(identity(42), 42);
    }
}
