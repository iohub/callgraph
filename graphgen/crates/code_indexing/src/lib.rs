pub mod graph;
mod misc;

extern crate serde;

use bincode;
use glob::glob;
use graph::*;
use log::{error, info, warn};
use misc::*;

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::io::prelude::*;
use tree_sitter::Node;
use tree_sitter::Parser;
use tree_sitter_typescript;

#[derive(Serialize, Deserialize, Debug, Clone)]
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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct CodeIndex {
    edges: BTreeMap<u64, Vec<u64>>,
    functions: BTreeMap<String, Function>,
    classes: BTreeMap<String, Class>,
    skip_dirs: Vec<String>,
    pub(crate) id_gen: IDGenerator,
}

impl CodeIndex {
    pub fn new() -> Self {
        CodeIndex {
            edges: BTreeMap::new(),
            functions: BTreeMap::new(),
            classes: BTreeMap::new(),
            skip_dirs: vec!["node_modules".to_string(), ".pnpm".to_string()],
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

    pub fn function_list(&self) -> Vec<String> {
        let mut result = vec![];
        for k in self.functions.keys() {
            result.push(k.clone())
        }
        result
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

    pub fn serde_tree(&mut self, funcname: &String, depth: i32) -> Option<GraphNode> {
        let id = self.id_gen.id(funcname);
        self._serde_tree_helper(id, depth)
    }

    fn _serde_tree_helper(&self, id: u64, depth: i32) -> Option<GraphNode> {
        if depth == 0 {
            return None;
        }
        let mut children = vec![];
        let mut value = 0;
        if let Some(outv) = self.edges.get(&id) {
            value = outv.len();
            for out in outv.iter() {
                if let Some(child) = self._serde_tree_helper(*out, depth - 1) {
                    children.push(child);
                }
            }
        }
        Some(GraphNode {
            name: self.id_gen.name(id).unwrap_or(&"nil".to_string()).clone(),
            children: children,
            value: value,
        })
    }

    pub fn into_file(&self, filename: &String) {
        let file = std::fs::File::create(filename).unwrap();
        bincode::serialize_into(file, self).unwrap();
    }

    pub fn parse_project(&mut self, dir: &String) -> Result<(), std::io::Error> {
        // TODO: supports more languages.
        let pattern = format!("{}/**/*.ts", dir);
        let entries = glob(&pattern).expect("Failed to read glob pattern");
        for entry in entries {
            match entry {
                Ok(path) => {
                    let path_str = path.display().to_string();
                    if !self.skip_dirs.iter().any(|s| path_str.contains(s)) {
                        if let Err(e) = self.parse_file(&path_str) {
                            error!("parse_file error {:?}", e);
                        }
                    }
                }
                Err(e) => {
                    error!("Read glob error {:?}", e);
                }
            }
        }

        Ok(())
    }

    pub fn parse_file(&mut self, filename: &String) -> Result<(), std::io::Error> {
        let content = std::fs::read_to_string(filename)?;
        let mut parser = Parser::new();
        parser
            .set_language(tree_sitter_typescript::language_typescript())
            .expect("Error loading TypeScript grammar");

        info!("parsing {}", filename);
        if let Some(tree) = parser.parse(&content, None) {
            let mut queue = vec![tree.root_node()];
            let mut cursor = tree.root_node().walk();
            while let Some(node) = queue.pop() {
                for child in node.children(&mut cursor) {
                    match child.kind() {
                        "class_declaration" => self.parse_class_declaration(child, &content),
                        "function_declaration" => self.parse_function_declaration(child, &content),
                        _ => {}
                    }
                    queue.push(child);
                }
            }
        }
        Ok(())
    }

    fn parse_function_declaration<'a>(&mut self, node: Node<'a>, content: &String) {
        let caller = str_by_field_name(node, "name", &content).unwrap();
        let function = Function {
            name: caller.clone(),
            pkg: "".to_string(),
            body: str_by_field_name(node, "body", &content).unwrap(),
        };
        self.add_function(&function);
        let calls = walk_collect(node, "call_expression");
        for call in calls {
            if let Some(callee) = str_by_field_name(call, "function", &content) {
                info!("{} -> {}", caller.clone(), callee);
                self.add_edge(&caller, &callee);
            }
        }
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
            let sig = Function {
                name: str_by_field_name(method, "name", &content).unwrap(),
                pkg: clsname.clone(),
                body: str_by_field_name(method, "body", &content).unwrap(),
            };
            self.add_function(&sig);
            let caller = sig.str();
            let calls = walk_collect(method, "call_expression");
            for call in calls {
                if let Some(callee) = str_by_field_name(call, "function", &content) {
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
    use env_logger;

    #[test]
    fn test_parse() {
        env_logger::init();
        let mut indexing = CodeIndex::new();
        let res = indexing.parse_file(&"../../tests/test0.txt".to_string());
        assert!(res.is_ok());
        assert!(indexing.classes.get("Parser").is_some());
    }

    #[test]
    fn test_load() {
        let mut indexing = CodeIndex::new();
        let res = indexing.parse_file(&"../../tests/test0.txt".to_string());
        assert!(res.is_ok());
        let datafile = "/tmp/code_index.bin".to_string();
        indexing.into_file(&datafile);
        let load_indexing = CodeIndex::load(&datafile);
        assert_eq!(load_indexing.edges.len(), indexing.edges.len());
    }
}
