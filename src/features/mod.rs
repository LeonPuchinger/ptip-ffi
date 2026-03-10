pub enum LanguageFeature {
    Function {
        name: String,
        args: Vec<String>,
        return_type: String,
    },
    Type {
        name: String,
        properties: Vec<(String, String)>,
    },
}
