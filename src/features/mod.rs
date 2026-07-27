pub enum PrimitiveType {
    Number,
    String,
    Boolean,
}

pub enum Type {
    Primitive(PrimitiveType),
    Composite { name: String, path: Vec<String> },
    Dynamic,
    Undefined,
}

pub struct FunctionParameter {
    pub name: String,
    pub r#type: Type,
    pub required: bool,
}

pub struct Function {
    pub name: String,
    pub positional_parameters: Vec<FunctionParameter>,
    pub named_parameters: Vec<FunctionParameter>,
    pub return_type: Type,
}

pub struct TypeDefinition {
    pub name: String,
    pub properties: Vec<(String, Type)>,
    pub default_constructor: Function,
}

pub struct Module {
    pub name: String,
    pub children: Vec<Module>,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDefinition>,
}
