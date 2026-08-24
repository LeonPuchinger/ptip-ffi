#[derive(Clone)]
pub struct ModulePath {
    pub segments: Vec<String>,
}

impl ModulePath {
    pub fn new(segments: Vec<&str>) -> Self {
        Self {
            segments: segments.into_iter().map(|s| s.to_string()).collect(),
        }
    }

    pub fn empty() -> Self {
        Self {
            segments: Vec::new(),
        }
    }

    pub fn leaf(&self) -> Option<&String> {
        self.segments.last()
    }

    pub fn format(&self, separator: &str) -> String {
        self.segments.join(separator)
    }
}

#[derive(Clone)]
pub struct Module {
    pub path: ModulePath,
    pub functions: Vec<FunctionDefinition>,
    pub types: Vec<TypeDefinition>,
}

#[derive(Clone)]
pub struct TypePath {
    pub module_path: ModulePath,
    pub name: String,
    pub type_arguments: Vec<Type>,
}

impl TypePath {
    pub fn new(module_path: ModulePath, name: String) -> Self {
        Self {
            module_path,
            name,
            type_arguments: Vec::new(),
        }
    }

    pub fn format(&self, module_separator: &str, type_separator: &str) -> String {
        std::format!(
            "{}{}{}",
            self.module_path.format(module_separator),
            type_separator,
            self.name
        )
    }
}

#[derive(Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Composite(TypePath),
    Array(Box<Type>),
    Tuple(Vec<Type>),
    Dynamic,
}

#[derive(Clone)]
pub enum PrimitiveType {
    Number,
    String,
    Boolean,
}

#[derive(Clone)]
pub struct TypeParameter {
    pub name: String,
    pub default: Option<Type>,
}

#[derive(Clone)]
pub struct AnonymousCallable {
    pub positional_parameters: Vec<ValueParameter>,
    pub named_parameters: Vec<ValueParameter>,
    pub return_type: Type,
    pub type_parameters: Vec<TypeParameter>,
}

#[derive(Clone)]
pub struct Method {
    pub name: String,
    pub r#static: bool,
    pub callable: AnonymousCallable,
}

#[derive(Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub properties: Vec<(String, Type)>,
    pub default_constructor: Option<AnonymousCallable>,
    pub named_constructors: Vec<FunctionDefinition>,
    pub methods: Vec<Method>,
    pub static_methods: Vec<Method>,
    pub type_parameters: Vec<TypeParameter>,
    pub implements: Vec<TypePath>,
}

#[derive(Clone)]
pub struct ValueParameter {
    pub name: String,
    pub r#type: Type,
    pub required: bool,
    pub variadic: bool,
    pub nullable: bool,
}

#[derive(Clone)]
pub struct FunctionDefinition {
    pub name: String,
    pub callable: AnonymousCallable,
}
