use hirune_image::contracts;
use serde_json::{Value, json};
#[test]
fn generated_schemas_validate_examples_and_openapi_references_resolve() {
    let docs = contracts::documents();
    let recipe = &docs
        .iter()
        .find(|(p, _)| p.ends_with("recipe-v1.schema.json"))
        .unwrap()
        .1;
    let validator = jsonschema::validator_for(recipe).unwrap();
    let example: Value =
        serde_json::from_str(include_str!("../contracts/examples/image-recipe-v1.json")).unwrap();
    assert!(validator.is_valid(&example));
    for invalid in [
        {
            let mut v = example.clone();
            v["contract_version"] = json!(2);
            v
        },
        {
            let mut v = example.clone();
            v["outputs"][0]["resize"]["fit"] = json!("cover");
            v
        },
        {
            let mut v = example.clone();
            v["outputs"][0]["encoding"]["effort"] = json!(7);
            v
        },
        {
            let mut v = example.clone();
            v["outputs"][1]["encoding"]["quality"] = json!(85);
            v
        },
        {
            let mut v = example.clone();
            v["unknown"] = json!(true);
            v
        },
    ] {
        assert!(!validator.is_valid(&invalid));
    }
    let document = contracts::openapi_document();
    fn walk(value: &Value, root: &Value) {
        match value {
            Value::Object(object) => {
                if let Some(reference) = object.get("$ref").and_then(Value::as_str) {
                    assert!(
                        reference.starts_with("#/components/schemas/"),
                        "unhoisted reference {reference}"
                    );
                    assert!(
                        root.pointer(&reference[1..]).is_some(),
                        "unresolved reference {reference}"
                    );
                }
                for child in object.values() {
                    walk(child, root)
                }
            }
            Value::Array(values) => {
                for child in values {
                    walk(child, root)
                }
            }
            _ => {}
        }
    }
    walk(&document, &document);
}
