use super::{
    json::JsonTraverser,
    toml::TomlTraverser,
    yaml::{YamlRoot, YamlTraverser},
};
use crate::{
    prelude::*,
    read_write::{filetype::FileType, traverser::Traversable},
};

enum Root<'r> {
    Json(fjson::ast::Root<'r>),
    Toml(toml_edit::Document),
    Yaml(YamlRoot),
}

pub struct Manager<'r> {
    root: Root<'r>,
}

impl<'r> Manager<'r> {
    pub fn new(ft: FileType, file_contents: &'r str) -> Result<Self, Zerr> {
        let root = match ft {
            FileType::Json => {
                Root::Json(fjson::ast::parse(file_contents).change_context(Zerr::InternalError)?)
            }
            FileType::Toml => {
                let root = file_contents
                    .parse::<toml_edit::Document>()
                    .change_context(Zerr::InternalError)?;
                Root::Toml(root)
            }
            FileType::Yaml => Root::Yaml(YamlRoot::new(file_contents)?),
        };

        Ok(Self { root })
    }

    pub fn rewrite(&self) -> Result<String, Zerr> {
        match &self.root {
            Root::Json(root) => {
                let mut jsonified = String::new();
                fjson::format::write_jsonc(&mut jsonified, root)
                    .change_context(Zerr::InternalError)?;
                Ok(jsonified)
            }
            Root::Toml(root) => Ok(root.to_string()),
            Root::Yaml(root) => Ok(root.file_contents.to_string()),
        }
    }

    pub fn traverser<'t>(&'t mut self) -> Result<Box<dyn Traversable<'r> + 't>, Zerr> {
        Ok(match &mut self.root {
            Root::Json(root) => Box::new(JsonTraverser::new(&mut root.value.token)),
            Root::Toml(root) => {
                let trav = TomlTraverser::new(root.as_item_mut().into());
                Box::new(trav)
            }
            Root::Yaml(root) => Box::new(YamlTraverser::new(root.build_active()?)),
        })
    }
}
