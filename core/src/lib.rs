pub static CORE_VERSION: &str = env!("CARGO_PKG_VERSION");
pub static RUSTC_VERSION: &str = env!("RUSTC_VERSION");

pub trait Function {
    fn call(&self, args: &[f64]) -> Result<f64, InvocationError>;

    /// Help text that may be used to display information about this function.
    fn help(&self) -> Option<&str> {
        None
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum InvocationError {
    InvalidArgumentCount { expected: usize, found: usize },
    Other { msg: String },
}

impl<S: ToString> From<S> for InvocationError {
    fn from(other: S) -> InvocationError {
        InvocationError::Other {
            msg: other.to_string(),
        }
    }
}

#[derive(Copy, Clone)]
pub struct PluginDeclaration {
    pub rustc_version: &'static str,
    pub core_version: &'static str,
    pub register: fn(&mut dyn PluginRegistrar),
}

pub trait PluginRegistrar {
    fn register_func(&mut self, name: &str, function: Box<dyn Function>);
}

#[macro_export]
macro_rules! export_plugin {
    ($register:expr) => {
        #[doc(hidden)]
        #[no_mangle]
        pub static plugin_declaration: $crate::PluginDeclaration = $crate::PluginDeclaration {
            rustc_version: $crate::RUSTC_VERSION,
            core_version: $crate::CORE_VERSION,
            register: $register,
        };
    };
}
