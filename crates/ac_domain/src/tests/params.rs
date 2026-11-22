use std::convert::Infallible;

use super::*;

fn filled(url: &str, values: &[(&str, &str)]) -> String {
    fill_path_params::<Infallible>(url, |name| {
        Ok(values
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, value)| (*value).to_owned()))
    })
    .unwrap()
}

#[test]
fn path_variables_are_found_in_the_path_only() {
    assert_eq!(
        path_param_names("{{baseUrl}}/users/:id/orders/:order_id?sort=:no#:frag"),
        ["id", "order_id"]
    );
    assert_eq!(path_param_names("http://localhost:8080/a/:x/b/:x"), ["x"]);
    assert_eq!(
        path_param_names("https://example.com/a/:/b"),
        Vec::<&str>::new()
    );
    assert_eq!(path_param_names("/:id.json"), ["id"]);
}

#[test]
fn path_variables_are_replaced_where_they_have_a_value() {
    assert_eq!(
        filled(
            "http://localhost:8080/users/:id/orders/:other?q=:id",
            &[("id", "42")]
        ),
        "http://localhost:8080/users/42/orders/:other?q=:id"
    );
    assert_eq!(filled("/:a/:a/:b", &[("a", "1"), ("b", "2")]), "/1/1/2");
    assert_eq!(filled("/:/x", &[("", "nope")]), "/:/x");
    assert_eq!(filled("no variables", &[("id", "1")]), "no variables");
}

#[test]
fn failures_of_a_value_stop_filling() {
    let result = fill_path_params("/:id", |_| Err("unknown"));
    assert_eq!(result, Err("unknown"));
}
