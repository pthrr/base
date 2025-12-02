pub mod verify_hot_path;

pub use verify_hot_path::{
    AllocationCheck, AtomicCheck, DivisionCheck, FunctionCallCheck, HotPathCheck, HotPathVerifier,
    IndirectionCheck, NonInboundsGepCheck, Severity, UnalignedAccessCheck, VolatileLoadCheck,
    VolatileStoreCheck, find_hot_functions_from_ir, verify_hot_function, verify_hot_path_functions,
};
