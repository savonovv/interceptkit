use crate::models::TransformOps;
use anyhow::Context;
use bytes::Bytes;
use http::HeaderMap;
use serde_json::Value;

pub fn apply_transform(
    headers: &mut HeaderMap,
    body: &mut Bytes,
    transform: &TransformOps,
) -> anyhow::Result<Vec<String>> {
    let mut notes = vec![];

    for (raw_name, raw_value) in &transform.set_headers {
        if let (Ok(header_name), Ok(header_value)) = (
            http::header::HeaderName::from_bytes(raw_name.as_bytes()),
            http::header::HeaderValue::from_str(raw_value),
        ) {
            headers.insert(header_name, header_value);
            notes.push(format!("setHeader:{}", raw_name.to_lowercase()));
        }
    }

    for raw_name in &transform.remove_headers {
        if let Ok(header_name) = http::header::HeaderName::from_bytes(raw_name.as_bytes()) {
            headers.remove(header_name);
            notes.push(format!("removeHeader:{}", raw_name.to_lowercase()));
        }
    }

    if let Some(replace_body) = &transform.replace_body {
        *body = Bytes::from(replace_body.clone().into_bytes());
        notes.push("replaceBody".to_string());
    }

    if !transform.json_set.is_empty() {
        let mut root: Value = if body.is_empty() {
            Value::Object(serde_json::Map::new())
        } else {
            serde_json::from_slice(body)
                .context("failed to parse body as JSON for jsonSet transform")?
        };

        let object = root
            .as_object_mut()
            .context("jsonSet requires JSON object body")?;

        for (key, value) in &transform.json_set {
            object.insert(key.clone(), value.clone());
            notes.push(format!("jsonSet:{}", key));
        }

        *body = Bytes::from(serde_json::to_vec(&root)?);

        if !headers.contains_key(http::header::CONTENT_TYPE) {
            headers.insert(
                http::header::CONTENT_TYPE,
                http::HeaderValue::from_static("application/json"),
            );
        }
    }

    let content_length_value = body.len().to_string();
    if let Ok(header_value) = http::HeaderValue::from_str(&content_length_value) {
        headers.insert(http::header::CONTENT_LENGTH, header_value);
    }

    Ok(notes)
}
