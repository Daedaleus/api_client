//! `$ref`s inside the description and example values built from schemas.

use serde_json::{Map, Value};

/// How deep nested schemas are followed for an example (cycles end here).
const MAX_DEPTH: usize = 6;
/// How many `$ref`s in a row are followed before giving up.
const MAX_REFS: usize = 16;

/// The whole description, for resolving `$ref`s.
pub(super) struct Document<'a> {
    root: &'a Value,
}

impl<'a> Document<'a> {
    pub(super) fn new(root: &'a Value) -> Self {
        Self { root }
    }

    /// `value`, or what its `$ref` (`#/components/...`) points to.
    pub(super) fn resolve<'v>(&'v self, mut value: &'v Value) -> &'v Value
    where
        'a: 'v,
    {
        for _ in 0..MAX_REFS {
            let Some(target) = value
                .get("$ref")
                .and_then(Value::as_str)
                .and_then(|reference| reference.strip_prefix('#'))
                .and_then(|pointer| self.root.pointer(&unescape(pointer)))
            else {
                return value;
            };
            value = target;
        }
        value
    }

    /// The security scheme `name` of `components.securitySchemes`.
    pub(super) fn scheme(&self, name: &str) -> Option<&'a Value> {
        self.root
            .get("components")?
            .get("securitySchemes")?
            .get(name)
    }

    /// An example value for `schema`: its own example, default or first
    /// enum value, else one built from its type.
    pub(super) fn example(&self, schema: &Value) -> Value {
        self.example_at(schema, 0)
    }

    fn example_at(&self, schema: &Value, depth: usize) -> Value {
        let schema = self.resolve(schema);
        for key in ["example", "default"] {
            if let Some(value) = schema.get(key) {
                return value.clone();
            }
        }
        if let Some(first) = schema
            .get("enum")
            .and_then(Value::as_array)
            .and_then(|values| values.first())
        {
            return first.clone();
        }
        if depth >= MAX_DEPTH {
            return Value::Null;
        }
        if let Some(parts) = schema.get("allOf").and_then(Value::as_array) {
            let mut merged = Map::new();
            for part in parts {
                if let Value::Object(object) = self.example_at(part, depth + 1) {
                    merged.extend(object);
                }
            }
            return Value::Object(merged);
        }
        for key in ["oneOf", "anyOf"] {
            if let Some(first) = schema
                .get(key)
                .and_then(Value::as_array)
                .and_then(|options| options.first())
            {
                return self.example_at(first, depth + 1);
            }
        }
        match type_of(schema) {
            Some("object") => Value::Object(
                properties(schema)
                    .map(|(name, property)| (name.clone(), self.example_at(property, depth + 1)))
                    .collect(),
            ),
            Some("array") => Value::Array(
                schema
                    .get("items")
                    .map_or_default(|items| vec![self.example_at(items, depth + 1)]),
            ),
            Some("string") => Value::String(string_example(schema).to_owned()),
            Some("integer" | "number") => Value::from(0),
            Some("boolean") => Value::Bool(false),
            _ => Value::Null,
        }
    }

    /// The properties of an object schema (following `$ref`s and `allOf`).
    pub(super) fn properties_of<'v>(&'v self, schema: &'v Value) -> Vec<(&'v String, &'v Value)>
    where
        'a: 'v,
    {
        let schema = self.resolve(schema);
        let mut all: Vec<_> = properties(schema).collect();
        for part in schema
            .get("allOf")
            .and_then(Value::as_array)
            .into_iter()
            .flatten()
        {
            all.extend(self.properties_of(part));
        }
        all
    }
}

/// The type of a schema; `object` if it only lists properties. In 3.1 the
/// type may be a list (`["string", "null"]`): the first that is not null.
fn type_of(schema: &Value) -> Option<&str> {
    match schema.get("type") {
        Some(Value::String(kind)) => Some(kind),
        Some(Value::Array(kinds)) => kinds
            .iter()
            .filter_map(Value::as_str)
            .find(|kind| *kind != "null"),
        _ => schema.get("properties").map(|_| "object"),
    }
}

fn properties(schema: &Value) -> impl Iterator<Item = (&String, &Value)> {
    schema
        .get("properties")
        .and_then(Value::as_object)
        .into_iter()
        .flatten()
}

/// A plausible string for the schema's `format`.
fn string_example(schema: &Value) -> &'static str {
    match schema.get("format").and_then(Value::as_str) {
        Some("date-time") => "2024-01-01T00:00:00Z",
        Some("date") => "2024-01-01",
        Some("uuid") => "00000000-0000-0000-0000-000000000000",
        Some("email") => "user@example.com",
        Some("uri" | "url") => "https://example.com",
        _ => "string",
    }
}

/// A JSON pointer from a `$ref`: URL-encoded spaces decoded (`~1` and `~0`
/// are handled by the pointer lookup itself).
fn unescape(pointer: &str) -> String {
    pointer.replace("%20", " ")
}

/// Text for a parameter or form field value.
pub(super) fn text(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::String(text) => text.clone(),
        other => other.to_string(),
    }
}
