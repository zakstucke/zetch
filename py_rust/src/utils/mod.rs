mod ordered_map_serializer;
pub mod user_input;
pub use ordered_map_serializer::ordered_map_serializer;
pub mod timing;

/// TODO remove with new ES
/// A generic trace_stack error to use when you don't want to create custom error types.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct AnyErr;

impl std::fmt::Display for AnyErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AnyErr")
    }
}

impl core::error::Error for AnyErr {}
