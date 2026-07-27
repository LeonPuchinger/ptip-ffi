#[derive(Clone)]
pub enum PrimitiveType {
    Number,
    String,
    Boolean,
}

#[derive(Clone)]
pub enum Type {
    Primitive(PrimitiveType),
    Composite { name: String, path: Vec<String> },
    Dynamic,
    Undefined,
}

#[derive(Clone)]
pub struct FunctionParameter {
    pub name: String,
    pub r#type: Type,
    pub required: bool,
}

#[derive(Clone)]
pub struct Function {
    pub name: String,
    pub positional_parameters: Vec<FunctionParameter>,
    pub named_parameters: Vec<FunctionParameter>,
    pub return_type: Type,
}

#[derive(Clone)]
pub struct TypeDefinition {
    pub name: String,
    pub properties: Vec<(String, Type)>,
    pub default_constructor: Function,
}

#[derive(Clone)]
pub struct Module {
    pub name: String,
    pub children: Vec<Module>,
    pub functions: Vec<Function>,
    pub types: Vec<TypeDefinition>,
}
