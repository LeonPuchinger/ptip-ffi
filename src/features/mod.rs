#[derive(Clone)]
pub struct ModulePath {
    pub segments: Vec<String>,
}

impl ModulePath {
    pub fn new(segments: Vec<String>) -> Self {
        Self { segments }
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
    pub children: Vec<Module>,
    pub functions: Vec<FunctionDefinition>,
    pub types: Vec<TypeDefinition>,
}

#[derive(Clone)]
pub struct TypePath {
    pub module_path: ModulePath,
    pub name: String,
}

impl TypePath {
    pub fn new(module_path: ModulePath, name: String) -> Self {
        Self { module_path, name }
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
pub struct TypeDefinition {
    pub name: String,
    pub properties: Vec<(String, Type)>,
    pub default_constructor: Option<FunctionDefinition>,
    pub named_constructors: Vec<FunctionDefinition>,
    pub methods: Vec<FunctionDefinition>,
    pub static_methods: Vec<FunctionDefinition>,
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
    pub positional_parameters: Vec<ValueParameter>,
    pub named_parameters: Vec<ValueParameter>,
    pub return_type: Type,
    pub type_parameters: Vec<TypeParameter>,
}
