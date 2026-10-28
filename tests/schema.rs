//! Validates the real document against `docs/json-schema.json`.
//!
//! The validator is deliberately small: it implements exactly the keywords the
//! schema uses, so the schema file stays the single description of the format
//! and the test does not grow a second one. Negative cases prove it can reject.

use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;

fn schema() -> Value {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("docs/json-schema.json");
    serde_json::from_str(&std::fs::read_to_string(path).expect("schema file")).expect("schema json")
}

/// Run the binary with a configuration directory that does not exist, so a
/// developer's own config cannot change the document under test.
fn document(args: &[&str]) -> Value {
    let isolated = std::env::temp_dir().join("jf-schema-suite-has-no-config-here");
    let out = Command::new(env!("CARGO_BIN_EXE_justfetch"))
        .args(args)
        .env("XDG_CONFIG_HOME", &isolated)
        .env("APPDATA", &isolated)
        .env("NO_COLOR", "1")
        .output()
        .expect("run justfetch");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    serde_json::from_slice(&out.stdout).expect("one JSON document")
}

fn type_matches(kind: &str, instance: &Value) -> bool {
    match kind {
        "object" => instance.is_object(),
        "array" => instance.is_array(),
        "string" => instance.is_string(),
        "integer" => instance.as_i64().is_some() || instance.as_u64().is_some(),
        "number" => instance.is_number(),
        "boolean" => instance.is_boolean(),
        "null" => instance.is_null(),
        other => panic!("schema uses an unsupported type: {other}"),
    }
}

/// Every keyword this validator implements. A schema that grows a constraint
/// outside this list must fail the test rather than be quietly ignored.
const IMPLEMENTED: [&str; 16] = [
    "$ref",
    "oneOf",
    "type",
    "const",
    "enum",
    "minimum",
    "maximum",
    "items",
    "required",
    "properties",
    "additionalProperties",
    // Document metadata, which constrains nothing.
    "$schema",
    "$id",
    "title",
    "description",
    "$defs",
];

fn validate_at(schema: &Value, root: &Value, instance: &Value, path: &str) -> Result<(), String> {
    if let Some(object) = schema.as_object() {
        for key in object.keys() {
            assert!(
                IMPLEMENTED.contains(&key.as_str()),
                "{path}: schema keyword `{key}` is not implemented by this validator"
            );
        }
    }
    if let Some(reference) = schema.get("$ref").and_then(Value::as_str) {
        let name = reference
            .strip_prefix("#/$defs/")
            .unwrap_or_else(|| panic!("unsupported $ref: {reference}"));
        let target = &root["$defs"][name];
        return validate_at(target, root, instance, path);
    }
    if let Some(variants) = schema.get("oneOf").and_then(Value::as_array) {
        let matches = variants
            .iter()
            .filter(|variant| validate_at(variant, root, instance, path).is_ok())
            .count();
        return if matches == 1 {
            Ok(())
        } else {
            Err(format!(
                "{path}: matched {matches} of {} variants",
                variants.len()
            ))
        };
    }
    if let Some(kind) = schema.get("type") {
        let kinds: Vec<&str> = match kind {
            Value::String(one) => vec![one.as_str()],
            Value::Array(many) => many.iter().filter_map(Value::as_str).collect(),
            _ => panic!("schema type must be a string or an array"),
        };
        if !kinds.iter().any(|kind| type_matches(kind, instance)) {
            return Err(format!("{path}: expected {kinds:?}, got {instance}"));
        }
    }
    if let Some(expected) = schema.get("const")
        && expected != instance
    {
        return Err(format!("{path}: expected const {expected}, got {instance}"));
    }
    if let Some(values) = schema.get("enum").and_then(Value::as_array)
        && !values.contains(instance)
    {
        return Err(format!("{path}: {instance} is not one of {values:?}"));
    }
    if let Some(minimum) = schema.get("minimum").and_then(Value::as_i64)
        && instance.as_i64().is_some_and(|value| value < minimum)
    {
        return Err(format!("{path}: {instance} is below {minimum}"));
    }
    if let Some(maximum) = schema.get("maximum").and_then(Value::as_i64)
        && instance.as_i64().is_some_and(|value| value > maximum)
    {
        return Err(format!("{path}: {instance} is above {maximum}"));
    }
    if let (Some(items), Some(array)) = (schema.get("items"), instance.as_array()) {
        for (index, element) in array.iter().enumerate() {
            validate_at(items, root, element, &format!("{path}[{index}]"))?;
        }
    }
    if let Some(object) = instance.as_object() {
        if let Some(required) = schema.get("required").and_then(Value::as_array) {
            for key in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(key) {
                    return Err(format!("{path}: missing required `{key}`"));
                }
            }
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
            for key in object.keys() {
                if !properties.is_some_and(|properties| properties.contains_key(key)) {
                    return Err(format!("{path}: unexpected property `{key}`"));
                }
            }
        }
        if let Some(properties) = properties {
            for (key, value) in object {
                if let Some(child) = properties.get(key) {
                    validate_at(child, root, value, &format!("{path}.{key}"))?;
                }
            }
        }
    }
    Ok(())
}

fn validate(instance: &Value) -> Result<(), String> {
    let root = schema();
    validate_at(&root, &root, instance, "$")
}

/// Walk every schema node, including ones no document happens to reach, and
/// assert each keyword is one the validator implements.
fn check_keywords(schema: &Value, path: &str) {
    let Some(object) = schema.as_object() else {
        return;
    };
    for key in object.keys() {
        assert!(
            IMPLEMENTED.contains(&key.as_str()),
            "{path}: schema keyword `{key}` is not implemented by this validator"
        );
    }
    if let Some(properties) = object.get("properties").and_then(Value::as_object) {
        for (name, child) in properties {
            check_keywords(child, &format!("{path}.properties.{name}"));
        }
    }
    for key in ["items", "additionalProperties"] {
        if let Some(child) = object.get(key) {
            check_keywords(child, &format!("{path}.{key}"));
        }
    }
    for key in ["oneOf", "anyOf", "allOf"] {
        if let Some(variants) = object.get(key).and_then(Value::as_array) {
            for (index, child) in variants.iter().enumerate() {
                check_keywords(child, &format!("{path}.{key}[{index}]"));
            }
        }
    }
    if let Some(defs) = object.get("$defs").and_then(Value::as_object) {
        for (name, child) in defs {
            check_keywords(child, &format!("{path}.$defs.{name}"));
        }
    }
}

#[test]
fn the_schema_uses_only_keywords_the_validator_implements() {
    let root = schema();
    check_keywords(&root, "$");
}

#[test]
fn the_schema_is_valid_json_and_lists_every_module() {
    let root = schema();
    let expected: Vec<String> = justfetch::module::ModuleId::ALL
        .iter()
        .map(|module| module.id().to_owned())
        .collect();
    let mut described: Vec<String> = root["properties"]
        .as_object()
        .expect("properties")
        .keys()
        .filter(|key| !matches!(key.as_str(), "schema_version" | "identity" | "environment"))
        .cloned()
        .collect();
    described.sort();
    let mut expected = expected;
    expected.sort();
    assert_eq!(described, expected, "the schema must describe every module");
}

#[test]
fn a_real_document_matches_the_schema() {
    validate(&document(&["--format", "json"])).unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn a_partial_document_matches_the_schema_too() {
    // The unselected modules exercise the `deselected` variant.
    validate(&document(&["--format", "json", "--modules", "os"]))
        .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn every_state_the_platform_can_report_validates() {
    // Ask for the modules whose states differ by platform; the documents above
    // already cover available and deselected on this host.
    validate(&document(&[
        "--format",
        "json",
        "--modules",
        "desktop,display,battery,gpu,packages,locale,local_ip,terminal",
    ]))
    .unwrap_or_else(|error| panic!("{error}"));
}

#[test]
fn the_validator_rejects_documents_that_break_the_contract() {
    let good = document(&["--format", "json", "--modules", "os"]);
    assert!(validate(&good).is_ok());

    let mut wrong_status = good.clone();
    wrong_status["os"] = serde_json::json!({"status": "maybe"});
    assert!(
        validate(&wrong_status).is_err(),
        "an unknown state must fail"
    );

    let mut missing_source = good.clone();
    missing_source["os"] = serde_json::json!({
        "status": "available",
        "value": {"family": "linux", "name": "x", "version": null, "id": null},
        "confidence": "confirmed",
    });
    assert!(
        validate(&missing_source).is_err(),
        "a missing source must fail"
    );

    let mut wrong_version = good.clone();
    wrong_version["schema_version"] = serde_json::json!(2);
    assert!(
        validate(&wrong_version).is_err(),
        "schema_version 2 must fail"
    );

    let mut extra_key = good.clone();
    extra_key["surprise"] = serde_json::json!({"status": "deselected"});
    assert!(
        validate(&extra_key).is_err(),
        "an extra top-level key must fail"
    );

    let mut bad_percent = good.clone();
    bad_percent["battery"] = serde_json::json!({
        "status": "available",
        "value": {"units": [{"name": "BAT0", "percent": 300, "state": "full"}], "power": "ac"},
        "source": "x",
        "confidence": "confirmed",
    });
    assert!(
        validate(&bad_percent).is_err(),
        "percent above 100 must fail"
    );
}
