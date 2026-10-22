use std::io::{Read, Write};
fn main() {
    let mut input = String::new();
    let result = std::io::stdin()
        .take(250_001)
        .read_to_string(&mut input)
        .map_err(|e| oj_runtime::Failure::new("IO_READ_FAILED", "Execution", e.to_string()))
        .and_then(|_| {
            serde_json::from_str(&input)
                .map_err(|e| oj_runtime::Failure::new("INPUT_INVALID", "Execution", e.to_string()))
        })
        .and_then(oj_runtime::handle);
    let response = match result {
        Ok(data) => serde_json::json!({"ok":true,"data":data}),
        Err(error) => serde_json::json!({"ok":false,"error":error}),
    };
    let _ = writeln!(std::io::stdout(), "{response}");
}
