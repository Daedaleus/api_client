use super::*;
use crate::command::Command;
use crate::test_support::{Fixture, changed, names};
use crate::text::keys;
use crate::view::TreeItemView;

fn duplicate(fixture: &Fixture, item_id: &str, name: &str) -> Command {
    Command::DuplicateItem {
        collection_id: fixture.collection.clone(),
        item_id: item_id.into(),
        name: name.into(),
    }
}

fn move_to(fixture: &Fixture, item_id: &str, parent_id: &str, index: usize) -> Command {
    Command::MoveItem {
        collection_id: fixture.collection.clone(),
        item_id: item_id.into(),
        parent_id: parent_id.into(),
        index,
    }
}

/// Ids of the folder and request in the tree, depth first.
fn ids(items: &[TreeItemView]) -> Vec<String> {
    items
        .iter()
        .flat_map(|item| match item {
            TreeItemView::Folder { id, items, .. } => {
                let mut all = vec![id.clone()];
                all.extend(ids(items));
                all
            }
            TreeItemView::Request { id, .. } => vec![id.clone()],
        })
        .collect()
}

#[tokio::test]
async fn copies_land_right_after_the_original_with_new_ids() {
    let fixture = Fixture::new();
    let (tree, created) = changed(fixture.run(duplicate(&fixture, &fixture.login, "Login copy")));
    assert_eq!(
        names(&tree.items),
        ["Auth/POST Login,POST Login copy", "GET Health"]
    );
    let copy = created.unwrap();
    assert_ne!(copy, fixture.login);
    assert_eq!(fixture.draft(&copy).url, "{{baseUrl}}/login");

    // A folder with everything in it.
    let (tree, created) = changed(fixture.run(duplicate(&fixture, &fixture.auth, "Auth copy")));
    assert_eq!(
        names(&tree.items),
        [
            "Auth/POST Login,POST Login copy",
            "Auth copy/POST Login,POST Login copy",
            "GET Health"
        ]
    );
    let all = ids(&tree.items);
    let mut unique = all.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), all.len(), "ids must not repeat: {all:?}");
    assert!(created.is_some());

    assert_eq!(
        fixture.fail(duplicate(&fixture, &fixture.login, "  ")),
        AppError::EmptyName
    );
    assert_eq!(
        fixture.fail(duplicate(&fixture, &ItemId::new().to_string(), "x")),
        AppError::UnknownItem
    );
}

#[tokio::test]
async fn items_move_within_and_between_folders() {
    let fixture = Fixture::new();
    let (tree, _) =
        changed(fixture.run(move_to(&fixture, &fixture.health, &fixture.collection, 0)));
    assert_eq!(names(&tree.items), ["GET Health", "Auth/POST Login"]);

    let (tree, _) = changed(fixture.run(move_to(&fixture, &fixture.health, &fixture.auth, 0)));
    assert_eq!(names(&tree.items), ["Auth/GET Health,POST Login"]);

    let (tree, _) = changed(fixture.run(move_to(&fixture, &fixture.login, &fixture.collection, 9)));
    assert_eq!(names(&tree.items), ["Auth/GET Health", "POST Login"]);
}

#[tokio::test]
async fn impossible_moves_are_refused() {
    let fixture = Fixture::new();
    assert_eq!(
        fixture.fail(move_to(&fixture, &fixture.auth, &fixture.auth, 0)),
        AppError::MoveIntoItself
    );
    assert_eq!(
        AppError::MoveIntoItself.user_text().key,
        keys::MOVE_INTO_ITSELF
    );
    assert_eq!(
        fixture.fail(move_to(&fixture, &fixture.login, &fixture.health, 0)),
        AppError::NotAFolder
    );
    assert_eq!(
        fixture.fail(move_to(
            &fixture,
            &ItemId::new().to_string(),
            &fixture.auth,
            0
        )),
        AppError::UnknownItem
    );

    // A folder into its own subfolder.
    let (_, inner) = changed(fixture.run(Command::CreateFolder {
        collection_id: fixture.collection.clone(),
        parent_id: fixture.auth.clone(),
        name: "Inner".into(),
    }));
    assert_eq!(
        fixture.fail(move_to(&fixture, &fixture.auth, &inner.unwrap(), 0)),
        AppError::MoveIntoItself
    );
}
