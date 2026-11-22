//! Parameters of a request beyond its URL text.
//!
//! - **Path variables:** a path segment `:name` (as in `/users/:id`) is
//!   replaced by the value of the path variable `name` when sending.
//! - **Query parameters** live in the URL as typed (`?page=2&q=tea`).
//!   Switched-off ones are kept with the request, outside the URL, so they
//!   are not sent but not lost either.

/// A path variable: replaces `:name` in the URL path.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PathParam {
    /// Name, without the colon.
    pub name: String,
    /// Value; may contain `{{variables}}`.
    pub value: String,
}

/// A query parameter that is switched off: kept, not sent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct QueryParam {
    /// Name as typed.
    pub name: String,
    /// Value as typed.
    pub value: String,
}

/// Where the path of `url` ends: at the query or the fragment.
fn path_end(url: &str) -> usize {
    url.find(['?', '#']).unwrap_or(url.len())
}

/// Characters of a path variable name.
fn is_name_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_' || c == '-'
}

/// The path variables of `url` in order of appearance (each name once):
/// segments that start with `:` right after a `/`, e.g. `id` in
/// `{{baseUrl}}/users/:id/orders`. A port (`host:8080`) is no variable.
#[must_use]
pub fn path_param_names(url: &str) -> Vec<&str> {
    let path = &url[..path_end(url)];
    let mut names: Vec<&str> = Vec::new();
    for (index, _) in path.match_indices("/:") {
        let start = index + 2;
        let length = path[start..]
            .find(|c: char| !is_name_char(c))
            .unwrap_or(path.len() - start);
        let name = &path[start..start + length];
        if !name.is_empty() && !names.contains(&name) {
            names.push(name);
        }
    }
    names
}

/// `url` with each path variable `:name` replaced by `value(name)`; names
/// without a value stay as they are.
///
/// # Errors
/// The first error of `value`.
pub fn fill_path_params<E>(
    url: &str,
    mut value: impl FnMut(&str) -> Result<Option<String>, E>,
) -> Result<String, E> {
    let end = path_end(url);
    let (path, rest) = url.split_at(end);
    let mut filled = String::with_capacity(url.len());
    let mut copied = 0;
    for (index, _) in path.match_indices("/:") {
        let start = index + 2;
        let length = path[start..]
            .find(|c: char| !is_name_char(c))
            .unwrap_or(path.len() - start);
        if length == 0 {
            continue;
        }
        if let Some(replacement) = value(&path[start..start + length])? {
            filled.push_str(&path[copied..=index]);
            filled.push_str(&replacement);
            copied = start + length;
        }
    }
    filled.push_str(&path[copied..]);
    filled.push_str(rest);
    Ok(filled)
}

#[cfg(test)]
#[path = "tests/params.rs"]
mod tests;
