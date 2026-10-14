use crate::model::SystemSnapshot;

pub fn render(snapshot: &SystemSnapshot) -> Result<String, serde_json::Error> {
    let mut document = serde_json::to_string_pretty(snapshot)?;
    document.push('\n');
    Ok(document)
}
