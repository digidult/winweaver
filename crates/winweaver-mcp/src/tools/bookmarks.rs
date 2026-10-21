//! The tools of the bookmarks: read always, changed when the user allows it.

use serde_json::{Value, json};
use winweaver_core::bookmarks::{Bookmark, Bookmarks};
use winweaver_core::clock;

use super::{Args, Kind, Needs, Outcome, Tool, ToolError, fail, s, schema};
use crate::{Server, State};

pub(super) const LIST_BOOKMARKS: Tool = Tool {
    name: "list_bookmarks",
    description: "The bookmarks of the user: options of profiles and keys of the registry, with their comments. The index names a bookmark for the other tools.",
    needs: Needs::READ,
    schema: || schema(json!({}), &[]),
    run: list_bookmarks,
};

pub(super) const ADD_BOOKMARK: Tool = Tool {
    name: "add_bookmark",
    description: "Adds a bookmark with a comment: of an option (kind option, item the id of a rule, path the profile file or empty) or of the registry (kind registry, path the key, item the value or empty). Needs the option edit_library.",
    needs: Needs::LIBRARY,
    schema: || {
        schema(
            json!({
                "kind": s("option or registry"),
                "item": s("The id of a rule, or the value of the key"),
                "path": s("The profile file of an option, or the key of the registry"),
                "comment": s("A comment")
            }),
            &["kind", "item"],
        )
    },
    run: add_bookmark,
};

pub(super) const SET_BOOKMARK_COMMENT: Tool = Tool {
    name: "set_bookmark_comment",
    description: "Changes the comment of a bookmark. Needs the option edit_library.",
    needs: Needs::LIBRARY,
    schema: || {
        schema(
            json!({ "index": { "type": "integer", "description": "The index of list_bookmarks" }, "comment": s("The comment") }),
            &["index", "comment"],
        )
    },
    run: set_bookmark_comment,
};

pub(super) const REMOVE_BOOKMARK: Tool = Tool {
    name: "remove_bookmark",
    description: "Removes a bookmark. Needs the option edit_library.",
    needs: Needs::LIBRARY,
    schema: || {
        schema(
            json!({ "index": { "type": "integer", "description": "The index of list_bookmarks" } }),
            &["index"],
        )
    },
    run: remove_bookmark,
};

fn load_marks(state: &State) -> Result<Bookmarks, ToolError> {
    Bookmarks::load_or_default(&state.options.marks_path)
        .map_err(|e| fail(Kind::ReadFailed, e.to_string()))
}

fn save_marks(state: &State, marks: &Bookmarks) -> Result<(), ToolError> {
    marks
        .save(&state.options.marks_path)
        .map_err(|e| fail(Kind::WriteFailed, e.to_string()))
}

/// The comment of a bookmark, at most 4000 characters without control characters but line breaks.
fn comment_arg(args: &Args<'_>) -> Result<String, ToolError> {
    let comment = args.optional_text("comment");
    if comment.chars().count() > 4000
        || comment
            .chars()
            .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err(fail(
            Kind::Invalid,
            "a comment has at most 4000 characters and no control characters",
        ));
    }
    Ok(comment)
}

/// The index of a bookmark of the list, checked against it.
fn index_arg(args: &Args<'_>, marks: &Bookmarks) -> Result<usize, ToolError> {
    let index = args
        .number("index")
        .ok_or_else(|| fail(Kind::Invalid, "index is a number of list_bookmarks"))?
        as usize;
    if index >= marks.items.len() {
        return Err(fail(Kind::NotFound, format!("no bookmark {index}")));
    }
    Ok(index)
}

fn list_bookmarks(_server: &Server, state: &mut State, _args: &Args<'_>) -> Outcome {
    let marks = load_marks(state)?;
    let items: Vec<Value> = marks
        .items
        .iter()
        .enumerate()
        .map(|(i, b)| {
            let kind = if b.is_registry() {
                "registry"
            } else if b.is_option() {
                "option"
            } else {
                b.document.as_str()
            };
            json!({ "index": i, "kind": kind, "item": b.item, "path": b.path, "comment": b.comment, "created": b.created })
        })
        .collect();
    Ok(json!({ "bookmarks": items }))
}

fn add_bookmark(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let item = args.optional_text("item");
    let path = args.optional_text("path");
    let mut mark = match args.text("kind")?.as_str() {
        "option" if !item.is_empty() => Bookmark::option(&item, &path),
        "registry" => {
            let key = winweaver_core::registry::normalize(&path)
                .ok_or_else(|| fail(Kind::Invalid, "path is not a key of the registry"))?;
            Bookmark::registry(&key, &item)
        }
        _ => {
            return Err(fail(
                Kind::Invalid,
                "kind is option (with an item) or registry",
            ));
        }
    };
    mark.comment = comment_arg(args)?;
    mark.created = clock::now_utc_iso();
    let mut marks = load_marks(state)?;
    if marks.find(&mark).is_some() {
        return Err(fail(Kind::Invalid, "this is bookmarked already"));
    }
    marks.items.push(mark);
    save_marks(state, &marks)?;
    Ok(json!({ "index": marks.items.len() - 1 }))
}

fn set_bookmark_comment(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let mut marks = load_marks(state)?;
    let index = index_arg(args, &marks)?;
    marks.items[index].comment = comment_arg(args)?;
    save_marks(state, &marks)?;
    Ok(json!({ "index": index, "done": "set_bookmark_comment" }))
}

fn remove_bookmark(_server: &Server, state: &mut State, args: &Args<'_>) -> Outcome {
    let mut marks = load_marks(state)?;
    let index = index_arg(args, &marks)?;
    marks.items.remove(index);
    save_marks(state, &marks)?;
    Ok(json!({ "index": index, "done": "remove_bookmark" }))
}
