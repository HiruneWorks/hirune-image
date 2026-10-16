fn main() {
    println!(
        "{}",
        serde_json::to_string_pretty(&hirune_image::contracts::openapi_document())
            .expect("OpenAPI")
    );
}
