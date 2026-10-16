use crate::http::{
    error::ErrorEnvelope,
    v1::{capabilities::Capabilities, images::contracts::RecipeInput},
};
use application::images::Manifest;
use serde_json::{Map, Value, json};
pub fn documents() -> Vec<(&'static str, Value)> {
    vec![
        (
            "contracts/image-recipe-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(RecipeInput)).expect("schema"),
        ),
        (
            "contracts/image-manifest-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(Manifest)).expect("schema"),
        ),
        (
            "contracts/error-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(ErrorEnvelope)).expect("schema"),
        ),
        (
            "contracts/capabilities-v1.schema.json",
            serde_json::to_value(schemars::schema_for!(Capabilities)).expect("schema"),
        ),
        ("docs/openapi.json", openapi_document()),
    ]
}
fn component<T: schemars::JsonSchema>(name: &str, components: &mut Map<String, Value>) {
    let schema = serde_json::to_value(schemars::schema_for!(T)).expect("schema");
    let mut schema: Value = serde_json::from_str(
        &serde_json::to_string(&schema)
            .expect("schema")
            .replace("#/$defs/", "#/components/schemas/"),
    )
    .expect("schema");
    if let Some(definitions) = schema
        .as_object_mut()
        .expect("object schema")
        .remove("$defs")
    {
        for (key, value) in definitions.as_object().expect("definitions") {
            if let Some(previous) = components.insert(key.clone(), value.clone()) {
                assert_eq!(previous, *value, "conflicting schema definition");
            }
        }
    }
    schema
        .as_object_mut()
        .expect("schema object")
        .remove("$schema");
    components.insert(name.into(), schema);
}
pub fn openapi_document() -> Value {
    let mut schemas = Map::new();
    component::<RecipeInput>("Recipe", &mut schemas);
    component::<Manifest>("Manifest", &mut schemas);
    component::<ErrorEnvelope>("Error", &mut schemas);
    component::<Capabilities>("Capabilities", &mut schemas);
    let mut responses = Map::new();
    responses.insert("200".into(),json!({"description":"JSON manifest first; binary WebP parts follow recipe order. Each part has Content-ID and Content-Length. All outputs are verified before response headers.","content":{"multipart/related":{"schema":{"type":"string","format":"binary"}}}}));
    for (status, description) in [
        ("400", "Invalid request or recipe"),
        ("413", "Byte/pixel/frame/output limit exceeded"),
        ("415", "Unsupported format"),
        ("422", "Invalid image or unsupported color/animation"),
        ("503", "Overloaded or unavailable; Retry-After: 1"),
        ("504", "Deadline exceeded before headers"),
        ("500", "Internal error"),
    ] {
        responses.insert(status.into(),json!({"description":description,"content":{"application/json":{"schema":{"$ref":"#/components/schemas/Error"}}}}));
    }
    json!({"openapi":"3.1.0","info":{"title":"Hirune Image","version":env!("CARGO_PKG_VERSION")},
        "paths":{
            "/health/live":{"get":{"responses":{"204":{"description":"Process alive"}}}},
            "/health/ready":{"get":{"responses":{"204":{"description":"Ready"},"503":{"description":"Shutting down or unavailable"}}}},
            "/v1/capabilities":{"get":{"responses":{"200":{"description":"Effective limits, pipeline policy and native codec versions","content":{"application/json":{"schema":{"$ref":"#/components/schemas/Capabilities"}}}}}}},
            "/v1/images/process":{"post":{"operationId":"processImage","requestBody":{"required":true,"content":{"multipart/form-data":{"schema":{"type":"object","additionalProperties":false,"required":["file","recipe"],"properties":{"file":{"type":"string","format":"binary"},"recipe":{"$ref":"#/components/schemas/Recipe"}}},"encoding":{"recipe":{"contentType":"application/json"}}}}},"responses":responses}}
        },"components":{"schemas":schemas}})
}
