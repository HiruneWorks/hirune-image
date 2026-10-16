fn main() -> anyhow::Result<()> {
    let check = std::env::args().any(|s| s == "--check");
    for (path, value) in hirune_image::contracts::documents() {
        let generated = format!("{}\n", serde_json::to_string_pretty(&value)?);
        if check {
            anyhow::ensure!(
                std::fs::read_to_string(path)? == generated,
                "contract drift: {path}; run just contracts"
            );
        } else {
            if let Some(parent) = std::path::Path::new(path).parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::write(path, generated)?;
        }
    }
    Ok(())
}
