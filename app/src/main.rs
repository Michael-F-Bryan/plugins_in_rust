use libloading::Library;
use plugins_core::{Function, InvocationError, PluginDeclaration};
use std::{
    alloc::System, collections::HashMap, env, ffi::OsStr, io, path::PathBuf,
    rc::Rc,
};

#[global_allocator]
static ALLOCATOR: System = System;

fn main() {
    // parse arguments
    let args = env::args().skip(1);
    let args = Args::parse(args)
        .expect("Usage: app <plugin-path> <function> <args>...");

    // create our functions table and load the plugin
    let mut functions = ExternalFunctions::new();

    unsafe {
        functions
            .load(&args.plugin_library)
            .expect("Function loading failed");
    }

    // then call the function
    let result = functions
        .call(&args.function, &args.arguments)
        .expect("Invocation failed");

    // print out the result
    println!(
        "{}({}) = {}",
        args.function,
        args.arguments
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(", "),
        result
    );
}

struct Args {
    plugin_library: PathBuf,
    function: String,
    arguments: Vec<f64>,
}

impl Args {
    fn parse(mut args: impl Iterator<Item = String>) -> Option<Args> {
        let plugin_library = PathBuf::from(args.next()?);
        let function = args.next()?;
        let mut arguments = Vec::new();

        for arg in args {
            arguments.push(arg.parse().ok()?);
        }

        Some(Args {
            plugin_library,
            function,
            arguments,
        })
    }
}

/// A map of all externally provided functions.
#[derive(Default)]
pub struct ExternalFunctions {
    functions: HashMap<String, FunctionProxy>,
    libraries: Vec<Rc<Library>>,
}

impl ExternalFunctions {
    pub fn new() -> ExternalFunctions { ExternalFunctions::default() }

    pub fn call(
        &self,
        function: &str,
        arguments: &[f64],
    ) -> Result<f64, InvocationError> {
        self.functions
            .get(function)
            .ok_or_else(|| format!("\"{}\" not found", function))?
            .call(arguments)
    }

    /// Load a plugin library and add all contained functions to the internal
    /// function table.
    ///
    /// # Safety
    ///
    /// A plugin library **must** be implemented using the
    /// [`plugins_core::plugin_declaration!()`] macro. Trying manually implement
    /// a plugin without going through that macro will result in undefined
    /// behaviour.
    pub unsafe fn load<P: AsRef<OsStr>>(
        &mut self,
        library_path: P,
    ) -> io::Result<()> {
        // load the library into memory
        let library = Rc::new(Library::new(library_path)?);

        // get a pointer to the plugin_declaration symbol.
        let decl = library
            .get::<*mut PluginDeclaration>(b"plugin_declaration\0")?
            .read();

        // version checks to prevent accidental ABI incompatibilities
        if decl.rustc_version != plugins_core::RUSTC_VERSION
            || decl.core_version != plugins_core::CORE_VERSION
        {
            return Err(io::Error::new(
                io::ErrorKind::Other,
                "Version mismatch",
            ));
        }

        let mut registrar = PluginRegistrar::new(Rc::clone(&library));

        (decl.register)(&mut registrar);

        // add all loaded plugins to the functions map
        self.functions.extend(registrar.functions);
        // and make sure ExternalFunctions keeps a reference to the library
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

    fn help(&self) -> Option<&str> { self.function.help() }
}
