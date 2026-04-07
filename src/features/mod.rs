pub struct Type {
    pub name: String,
    pub path: Vec<String>,
}

pub struct FunctionParameter {
    pub name: String,
    pub r#type: Option<Type>,
    pub required: bool,
}

pub enum LanguageFeature {
    Function {
        name: String,
        args: Vec<FunctionParameter>,
        return_type: String,
    },
    Type {
        name: String,
        properties: Vec<(String, String)>,
    },
}
