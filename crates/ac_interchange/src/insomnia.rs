//! Insomnia exports (format 4, JSON or YAML) as collections.
//!
//! The export is a flat list of resources linked by `parentId`: the first
//! workspace becomes the collection, its request groups folders (with their
//! variables), its requests requests – ordered by Insomnia's sort key. The
//! base environment gives the collection's variables, its sub-environments
//! the environments. Insomnia's `{{ _.name }}` becomes `{{name}}`; template
//! tags (`{% response … %}`, `{% faker … %}`) have no counterpart, stay as
//! text and are reported. Scripts are taken over as written: the script
//! API knows `insomnia` (the same object as Postman's `pm`).

mod request;

use ac_domain::collection::{Collection, Folder, Item, ItemId};
use ac_domain::environment::{Environment, Variable};
use ac_domain::script::Scripts;
use ac_domain::tls::CertificateCheck;
use ac_domain::transport::TransportSettings;
use serde_json::Value;

use crate::import::{Imported, Skipped, SkippedKind, text_of};

/// Name of a collection without a workspace.
const UNNAMED: &str = "Insomnia";
/// `_type` of the export itself.
const EXPORT: &str = "export";
/// Resource types (`_type`).
mod kind {
    pub const WORKSPACE: &str = "workspace";
    pub const GROUP: &str = "request_group";
    pub const REQUEST: &str = "request";
    pub const ENVIRONMENT: &str = "environment";
    pub const GRPC: &str = "grpc_request";
    pub const WEBSOCKET: &str = "websocket_request";
}

/// Converts an Insomnia export; `None` if `root` is none.
pub(crate) fn from_value(root: &Value) -> Option<Imported> {
    if root.get("_type").and_then(Value::as_str) != Some(EXPORT) {
        return None;
    }
    let resources = Resources(root.get("resources")?.as_array()?);
    let workspace = resources.of_kind(kind::WORKSPACE).next();
    let name = workspace
        .and_then(|workspace| text_of(workspace.get("name")))
        .unwrap_or_else(|| UNNAMED.to_owned());
    let root_id = workspace.and_then(|workspace| text_of(workspace.get("_id")));
    let mut skipped = Vec::new();
    let items = match &root_id {
        Some(id) => resources.items_in(id, &mut skipped),
        None => resources.orphans(&mut skipped),
    };
    let base = root_id.as_deref().and_then(|id| {
        resources
            .children(id)
            .find(|resource| kind_of(resource) == kind::ENVIRONMENT)
    });
    let environments = base
        .and_then(|base| text_of(base.get("_id")))
        .map_or_default(|base_id| {
            resources
                .children(&base_id)
                .filter(|resource| kind_of(resource) == kind::ENVIRONMENT)
                .map(|environment| Environment {
                    variables: variables(environment.get("data")),
                    ..Environment::new(text_of(environment.get("name")).unwrap_or_default())
                })
                .collect()
        });
    let collection = Collection {
        id: ItemId::new(),
        items,
        variables: base.map_or_default(|base| variables(base.get("data"))),
        auth: ac_domain::auth::Auth::Inherit,
        certificate_check: CertificateCheck::Inherit,
        transport: TransportSettings::default(),
        scripts: Scripts::default(),
        name,
    };
    Some(Imported {
        collection,
        environments,
        skipped,
    })
}

/// The resources of the export.
struct Resources<'a>(&'a [Value]);

impl<'a> Resources<'a> {
    fn of_kind(&self, wanted: &'a str) -> impl Iterator<Item = &'a Value> + 'a {
        self.0
            .iter()
            .filter(move |resource| kind_of(resource) == wanted)
    }

    /// The resources whose parent is `id`, in Insomnia's order.
    fn children(&self, id: &str) -> impl Iterator<Item = &'a Value> + 'a {
        let mut children: Vec<&Value> = self
            .0
            .iter()
            .filter(|resource| text_of(resource.get("parentId")).as_deref() == Some(id))
            .collect();
        children.sort_by(|a, b| sort_key(a).total_cmp(&sort_key(b)));
        children.into_iter()
    }

    /// Folders and requests under `id`.
    fn items_in(&self, id: &str, skipped: &mut Vec<Skipped>) -> Vec<Item> {
        self.children(id)
            .filter_map(|resource| self.item_of(resource, skipped))
            .collect()
    }

    /// Without a workspace: the folders and requests whose parent is no
    /// folder.
    fn orphans(&self, skipped: &mut Vec<Skipped>) -> Vec<Item> {
        let groups: Vec<String> = self
            .of_kind(kind::GROUP)
            .filter_map(|group| text_of(group.get("_id")))
            .collect();
        let mut top: Vec<&Value> = self
            .0
            .iter()
            .filter(|resource| {
                text_of(resource.get("parentId")).is_none_or(|parent| !groups.contains(&parent))
            })
            .collect();
        top.sort_by(|a, b| sort_key(a).total_cmp(&sort_key(b)));
        top.into_iter()
            .filter_map(|resource| self.item_of(resource, skipped))
            .collect()
    }

    fn item_of(&self, resource: &Value, skipped: &mut Vec<Skipped>) -> Option<Item> {
        let name = text_of(resource.get("name")).unwrap_or_default();
        match kind_of(resource) {
            kind::GROUP => {
                let id = text_of(resource.get("_id")).unwrap_or_default();
                Some(Item::Folder(Folder {
                    id: ItemId::new(),
                    items: self.items_in(&id, skipped),
                    variables: variables(resource.get("environment")),
                    auth: request::auth(resource.get("authentication"), &name, skipped),
                    certificate_check: CertificateCheck::Inherit,
                    transport: TransportSettings::default(),
                    scripts: request::scripts(resource),
                    name,
                }))
            }
            kind::REQUEST => Some(Item::Request(request::request_of(resource, skipped))),
            other @ (kind::GRPC | kind::WEBSOCKET) => {
                skipped.push(Skipped {
                    item: name,
                    what: SkippedKind::RequestKind(
                        if other == kind::GRPC {
                            "gRPC"
                        } else {
                            "WebSocket"
                        }
                        .to_owned(),
                    ),
                });
                None
            }
            _ => None,
        }
    }
}

fn kind_of(resource: &Value) -> &str {
    resource
        .get("_type")
        .and_then(Value::as_str)
        .unwrap_or_default()
}

fn sort_key(resource: &Value) -> f64 {
    resource
        .get("metaSortKey")
        .and_then(Value::as_f64)
        .unwrap_or_default()
}

/// The variables of an environment's `data` object; nested values as JSON.
fn variables(data: Option<&Value>) -> Vec<Variable> {
    data.and_then(Value::as_object).map_or_default(|data| {
        data.iter()
            .map(|(name, value)| {
                let value = text_of(Some(value)).unwrap_or_else(|| match value {
                    Value::Null => String::new(),
                    other => other.to_string(),
                });
                Variable::plain(name.clone(), convert_template(&value).0)
            })
            .collect()
    })
}

/// Insomnia's template text in this program's syntax – `{{ _.name }}` and
/// `{{ name }}` as `{{name}}` – and the names of the template tags
/// (`{% response … %}`) found, which stay as written.
pub(crate) fn convert_template(text: &str) -> (String, Vec<String>) {
    let mut converted = String::with_capacity(text.len());
    let mut tags = Vec::new();
    let mut rest = text;
    loop {
        let variable = rest.find("{{");
        let tag = rest.find("{%");
        match (variable, tag) {
            (Some(start), tag) if tag.is_none_or(|tag| start < tag) => {
                let Some(length) = rest[start..].find("}}") else {
                    break;
                };
                let inner = rest[start + 2..start + length].trim();
                let name = inner.strip_prefix("_.").unwrap_or(inner);
                converted.push_str(&rest[..start]);
                converted.push_str("{{");
                converted.push_str(name);
                converted.push_str("}}");
                rest = &rest[start + length + 2..];
            }
            (_, Some(start)) => {
                let Some(length) = rest[start..].find("%}") else {
                    break;
                };
                let inner = rest[start + 2..start + length].trim();
                if let Some(tag) = inner.split_whitespace().next() {
                    tags.push(tag.to_owned());
                }
                converted.push_str(&rest[..start + length + 2]);
                rest = &rest[start + length + 2..];
            }
            _ => break,
        }
    }
    converted.push_str(rest);
    (converted, tags)
}

#[cfg(test)]
#[path = "tests/insomnia.rs"]
mod tests;
