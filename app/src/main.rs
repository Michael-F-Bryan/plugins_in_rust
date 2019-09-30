use libloading::Library;
use plugins_core::{Function, InvocationError, PluginDeclaration};
use std::{collections::HashMap, ffi::OsStr, io, rc::Rc};

fn main() {
    println!("Hello, world!");
}

/// A map of all externally provided functions.
#[derive(Default)]
pub struct ExternalFunctions {
    functions: HashMap<String, FunctionProxy>,
    libraries: Vec<Rc<Library>>,
}

impl ExternalFunctions {
    pub fn new() -> ExternalFunctions {
        ExternalFunctions::default()
    }

    pub fn load<P: AsRef<OsStr>>(&mut self, library_path: P) -> io::Result<()> {
        let library = Rc::new(Library::new(library_path)?);

        let decl = unsafe { library.get::<PluginDeclaration>(b"plugin_declaration\0")? };

        if decl.rustc_version != plugins_core::RUSTC_VERSION
            || decl.core_version != plugins_core::CORE_VERSION
        {
            return Err(io::Error::new(io::ErrorKind::Other, "Version mismatch"));
        }

        let mut registrar = PluginRegistrar::new(Rc::clone(&library));

        unsafe {
            (decl.register)(&mut registrar);
        }

        self.functions.extend(registrar.functions);
        self.libraries.push(library);

        Ok(())
    }
}

struct PluginRegistrar {
    functions: HashMap<String, FunctionProxy>,
    lib: Rc<Library>,
}

impl PluginRegistrar {
    fn new(lib: Rc<Library>) -> PluginRegistrar {
        PluginRegistrar {
            lib,
            functions: HashMap::default(),
        }
    }
}

impl plugins_core::PluginRegistrar for PluginRegistrar {
    fn register_function(&mut self, name: &str, function: Box<dyn Function>) {
        let proxy = FunctionProxy {
            function,
            _lib: Rc::clone(&self.lib),
        };
        self.functions.insert(name.to_string(), proxy);
    }
}

/// A proxy object which wraps a [`Function`] and makes sure it can't outlive
/// the library it came from.
pub struct FunctionProxy {
    function: Box<dyn Function>,
    _lib: Rc<Library>,
}

impl Function for FunctionProxy {
    fn call(&self, args: &[f64]) -> Result<f64, InvocationError> {
        self.function.call(args)
    }

    fn help(&self) -> Option<&str> {
        self.function.help()
    }
}
