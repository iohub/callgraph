mod misc;

extern crate serde;

use bincode;
use misc::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::prelude::*;
use tree_sitter::Node;
use tree_sitter::Parser;
use tree_sitter_typescript;

#[derive(Serialize, Deserialize, Debug)]
struct IDGenerator {
    id_map: BTreeMap<String, u64>,
    name_map: BTreeMap<u64, String>,
    next_id: u64,
}

impl IDGenerator {
    fn new() -> Self {
        IDGenerator {
            id_map: BTreeMap::new(),
            name_map: BTreeMap::new(),
            next_id: 1,
        }
    }

    pub fn id(&mut self, sig: &String) -> u64 {
        *self.id_map.entry(sig.clone()).or_insert_with(|| {
            let id = self.next_id;
            self.next_id += 1;
            self.name_map.insert(id, sig.clone());
            id
        })
    }

    pub fn name(&self, id: u64) -> Option<&String> {
        self.name_map.get(&id)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Signature {
    name: String,
    pkg: String,
    body: String,
}

impl Signature {
    pub fn str(&self) -> String {
        if self.pkg == "" {
            return self.name.clone();
        }
        format!("{}.{}", self.pkg, self.name)
    }
    pub fn new(name: String, pkg: String) -> Self {
        Signature {
            name: name,
            pkg: pkg,
            body: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CodeIndex {
    edges: BTreeMap<u64, Vec<u64>>,
    signatures: BTreeMap<String, Signature>,
    id_gen: IDGenerator,
}

impl CodeIndex {
    pub fn new() -> Self {
        CodeIndex {
            edges: BTreeMap::new(),
            signatures: BTreeMap::new(),
            id_gen: IDGenerator::new(),
        }
    }

    pub fn load(filename: &String) -> Self {
        let mut file = std::fs::File::open(filename).unwrap();
        let mut buffer = Vec::new();
        file.read_to_end(&mut buffer).unwrap();
        let data: CodeIndex = bincode::deserialize(&buffer).unwrap();
        data
    }

    fn add_function(&mut self, sig: &Signature) {
        self.signatures.insert(sig.str(), sig.clone());
    }

    fn add_edge(&mut self, from: &String, to: &String) {
        let from_id = self.id_gen.id(from);
        let to_id: u64 = self.id_gen.id(to);
        match self.edges.get_mut(&from_id) {
            Some(v) => v.push(to_id),
            None => {
                self.edges.insert(from_id, vec![to_id]);
            }
        }
    }

    pub fn into_file(&self, filename: &String) {
        let file = std::fs::File::create(filename).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn parse_ts(&mut self, filename: &String) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(filename)?;
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        if let Some(tree) = parser.parse(&content, None) {
            let classes = walk_collect(tree.root_node(), "class_declaration");
            for cls in classes.iter() {
                self.parse_class_declaration(*cls, &content);
            }
        }
        Ok(())
    }

    fn parse_class_declaration<'a>(&mut self, node: Node<'a>, content: &String) {
        let methods = walk_collect(node, "method_definition");
        let clsname = str_by_field_name(node, "name", &content).unwrap_or("".to_string());
        let clsdot = clsname.clone() + ".";
        for method in methods {
            let caller = format!(
                "{}.{}",
                clsname,
                str_by_field_name(method, "name", &content).unwrap_or("Nil".to_string())
            );

            let calls = walk_collect(method, "call_expression");
            for call in calls {
                if let Some(callee) = str_by_field_name(call, "function", &content) {
                    // println!("{} -> {}", caller, callee.replace("this.", &clsdot));
                    let _callee = callee.replace("this.", &clsdot);
                    self.add_edge(&caller, &_callee);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse() {
        let mut indexing = CodeIndex::new();
        let res = indexing.parse_ts(&"../../tests/test0.ts".to_string());
        assert!(res.is_ok());
    }
}
