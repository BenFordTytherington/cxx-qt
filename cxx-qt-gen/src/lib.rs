mod extract;
mod gen_cpp;
mod gen_rs;

pub use extract::{extract_qobject, QObject};
pub use gen_cpp::{generate_format, generate_qobject_cpp, CppObject};
pub use gen_rs::{generate_qobject_cxx, generate_qobject_rs};

/// The complete contents of the "rust/cxx_qt.h" header.
pub static HEADER: &str = include_str!("cxx_qt.h");