/// Simplifies creating path errs:
macro_rules! raise_invalid_path {
    ($path:expr, $cur_index:expr, $parent:expr) => {
        zerr!(
            Zerr::FilePathError,
            "Invalid key '{}' at path location '{}'. Parent value below.",
            $path[$cur_index],
            $path[..$cur_index].join(".")
        )
        .attach_printable($parent)
    };
}

pub(crate) use raise_invalid_path;
