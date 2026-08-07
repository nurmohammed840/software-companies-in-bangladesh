use super::*;
use markup5ever_rcdom::NodeData;

pub fn normalize_markdown_from(html: &str) -> Result<String> {
    let converter = htmd::HtmlToMarkdown::builder()
        .skip_tags(vec![
            "script", "style", "noscript", // Executable / styling
            "iframe", "canvas", "svg", "object", "embed", // Embedded content
            "img", "picture", "source", "video", "audio", // Media
            "head", "form", "footer", "aside", "template", // Other
        ])
        .build();

    let node = converter.html_to_tree(html)?;
    clean_markdown(&node);
    Ok(converter.tree_to_markdown(&node))
}

pub fn is_empty(node: &htmd::Node) -> bool {
    node.children
        .borrow()
        .iter()
        .all(|child| match &child.data {
            NodeData::Text { contents } => contents.borrow().trim().is_empty(),
            NodeData::Comment { .. } => true,
            NodeData::Element { .. } => is_empty(child),
            _ => false,
        })
}

pub fn clean_markdown(node: &htmd::Node) {
    node.children.borrow_mut().retain(|child| {
        match &child.data {
            NodeData::Comment { .. } => false,
            NodeData::Text { contents } => !contents.borrow().trim().is_empty(),
            NodeData::Element { attrs, .. } => {
                if attrs
                    .borrow()
                    .iter()
                    .find(|attr| attr.name.local.as_ref() == "href")
                    .map(|attr| attr.value.as_ref())
                    .is_some_and(|link| link.is_empty() || link.trim().starts_with('#'))
                {
                    return false;
                }

                if is_empty(child) {
                    // Remove empty items.
                    return false;
                }

                true
            }
            _ => true,
        }
    });

    for child in node.children.borrow().iter() {
        clean_markdown(child);
    }
}
