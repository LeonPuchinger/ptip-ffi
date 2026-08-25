use std::path::PathBuf;

use crate::{codegen::CodegenOutput, features::Module};

pub fn generate_callee(_modules: Vec<&Module>) -> Vec<CodegenOutput> {
    vec![CodegenOutput {
        path: PathBuf::from("main.py"),
        content: "import os\nimport socket\n\n\ndef serve(socket_path: str) -> None:\n    if os.path.exists(socket_path):\n        os.unlink(socket_path)\n    server = socket.socket(socket.AF_UNIX, socket.SOCK_STREAM)\n    server.bind(socket_path)\n    server.listen(1)\n    try:\n        while True:\n            connection, _ = server.accept()\n            with connection:\n                connection.recv(1)\n    finally:\n        server.close()\n\n\nif __name__ == '__main__':\n    serve('/tmp/ptip-ffi-python.sock')\n".to_string(),
    }]
}