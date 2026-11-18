//! Texto de un JSON o YAML para el índice full-text: cada valor como `ruta.de.claves: valor`, así
//! se busca por clave con su contexto (`paths./transfers.post.summary: Envía una transferencia`)
//! en vez de por llaves, comillas y sangrías. Si no se puede interpretar, se indexa el texto crudo.

use serde_json::Value;

pub fn json_text(text: &str) -> Option<String> {
    serde_json::from_str::<Value>(text)
        .ok()
        .map(|v| flatten(&v))
}

pub fn yaml_text(text: &str) -> Option<String> {
    // Varios documentos separados por `---` (p. ej. manifiestos de Kubernetes)
    let mut out = Vec::new();
    for doc in serde_yaml::Deserializer::from_str(text) {
        let value: Value = serde::Deserialize::deserialize(doc).ok()?;
        out.push(flatten(&value));
    }
    Some(out.join("\n"))
}

fn flatten(value: &Value) -> String {
    let mut lines = Vec::new();
    walk(value, &mut String::new(), &mut lines);
    lines.join("\n")
}

fn walk(value: &Value, path: &mut String, out: &mut Vec<String>) {
    match value {
        Value::Object(map) => {
            for (k, v) in map {
                let len = path.len();
                if !path.is_empty() {
                    path.push('.');
                }
                path.push_str(k);
                walk(v, path, out);
                path.truncate(len);
            }
        }
        // En los arreglos no se repite el índice: la ruta de cada elemento es la del arreglo
        Value::Array(items) => items.iter().for_each(|v| walk(v, path, out)),
        Value::Null => {}
        Value::String(s) if s.trim().is_empty() => {}
        Value::String(s) => out.push(format!("{path}: {s}")),
        other => out.push(format!("{path}: {other}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn aplana_json_y_yaml_con_su_ruta() {
        let json = r#"{"info":{"title":"API Transferencias","version":"1.0"},"tags":[{"name":"pagos"}],"x":null}"#;
        assert_eq!(
            json_text(json).unwrap(),
            "info.title: API Transferencias\ninfo.version: 1.0\ntags.name: pagos"
        );
        let yaml = "kind: Deployment\nmetadata:\n  name: ach-operations\n---\nkind: Service\nspec:\n  ports:\n    - port: 8080\n";
        assert_eq!(
            yaml_text(yaml).unwrap(),
            "kind: Deployment\nmetadata.name: ach-operations\nkind: Service\nspec.ports.port: 8080"
        );
        assert!(json_text("{ roto").is_none());
    }
}
