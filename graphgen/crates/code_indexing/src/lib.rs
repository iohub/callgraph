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
struct Class {
    name: String,
    declaration: String,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Function {
    name: String,
    pkg: String,
    body: String,
}

impl Function {
    pub fn str(&self) -> String {
        if self.pkg == "" {
            return self.name.clone();
        }
        format!("{}.{}", self.pkg, self.name)
    }
    pub fn new(name: String, pkg: String) -> Self {
        Function {
            name: name,
            pkg: pkg,
            body: "".to_string(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
struct CodeIndex {
    edges: BTreeMap<u64, Vec<u64>>,
    functions: BTreeMap<String, Function>,
    classes: BTreeMap<String, Class>,
    id_gen: IDGenerator,
}

impl CodeIndex {
    pub fn new() -> Self {
        CodeIndex {
            edges: BTreeMap::new(),
            functions: BTreeMap::new(),
            classes: BTreeMap::new(),
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

    fn add_function(&mut self, func: &Function) {
        self.functions
            .entry(func.str())
            .or_insert_with(|| func.clone());
    }

    fn add_class(&mut self, cls: &Class) {
        self.classes
            .entry(cls.name.clone())
            .or_insert_with(|| cls.clone());
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

    pub fn parse_file(&mut self, filename: &String) -> Result<(), std::io::Error> {
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
        let clsname = str_by_field_name(node, "name", &content).unwrap_or("".to_string());
        let clsdot = clsname.clone() + ".";
        let methods = walk_collect(node, "method_definition");
        let end_byte = if let Some(first) = methods.first() {
            first.start_byte()
        } else {
            node.end_byte()
        };

        if let Some(declaration) = substr(content, node.start_byte(), end_byte) {
            self.add_class(&Class {
                name: clsname.clone(),
                declaration: declaration,
            });
        }

        for method in methods {
            let sig = Function::new(
                clsname.clone(),
                str_by_field_name(method, "name", &content).unwrap_or("Nil".to_string()),
            );
            self.add_function(&sig);
            let caller = sig.str();
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
        let res = indexing.parse_file(&"../../tests/test0.ts".to_string());
        assert!(res.is_ok());
        assert!(indexing.classes.get("Parser").is_some());
    }

    #[test]
    fn test_load() {
        let mut indexing = CodeIndex::new();
        let res = indexing.parse_file(&"../../tests/test0.ts".to_string());
        assert!(res.is_ok());
        let datafile = "/tmp/code_index.bin".to_string();
        indexing.into_file(&datafile);
        let load_indexing = CodeIndex::load(&datafile);
        assert_eq!(load_indexing.edges.len(), indexing.edges.len());
    }
}
