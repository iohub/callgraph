use tree_sitter::Node;

pub(crate) fn str_by_field_name<'a>(
    node: Node<'a>,
    field: &str,
    content: &String,
) -> Option<String> {
    match node.child_by_field_name(field) {
        None => return None,
        Some(child) => {
            let bytes = content.as_bytes();
            return String::from_utf8(bytes[child.start_byte()..child.end_byte()].to_vec()).ok();
        }
    }
}

pub(crate) fn walk_collect<'a>(node: Node<'a>, kind: &str) -> Vec<Node<'a>> {
    let mut result = vec![];
    let mut queue = vec![node];
    let mut cursor = node.walk();
    while let Some(node) = queue.pop() {
        for child in node.children(&mut cursor) {
            if child.kind() == kind {
                result.push(child);
            }
            queue.push(child);
        }
    }
    result
}
