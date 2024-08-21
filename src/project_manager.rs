enum Accessory {
    Text,
    Header,
    Todo { checked: bool },
    Table { data: Vec<Vec<String>> },
}

struct Node {
    text: String,
    children: Vec<Node>,
    accessory: Accessory,
}

struct Project {
    parent_node: Node,
}

pub fn handle_keystroke(project: &mut Project, kstr: &str) -> bool {
    match kstr {
        "C-C" -> {
            project.parent_node
        }
    }
}

pub fn fill_buffer(project: &Project) -> Vec<String> {
    let mut buf = Vec::new();
    fill_buffer_recursive(project.parent_node, &mut buf);
}

fn fill_buffer_recursive(node: &Node, buf: &mut Vec<String>) {
    buf.push_str(&node.text);
    buf.push('\n');
    // TODO render accessories
    for child in &node.children {
        fill_buffer_recursive(child, buf);
    }
}


// Actions
//
//
fn add_node() {
}

fn remove_node() {
}

fn move_node_up() {
}

fn move_node_down() {
}

fn promote_node() {
}

fn demote_node() {
}
