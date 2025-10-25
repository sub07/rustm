use std::path::PathBuf;

use assert_matches::debug_assert_matches;
use toml_edit::{DocumentMut, Item, Value};

use crate::project::Project;

pub struct ManifestEditor {
    data: DocumentMut,
    manifest_path: PathBuf,
}

#[derive(Debug)]
enum DepFormat {
    Simple,
    Detailed,
}

#[easy_ext::ext]
impl Value {
    fn fmt(&mut self) {
        match self {
            Self::String(s) => s.fmt(),
            Self::Integer(i) => i.fmt(),
            Self::Float(f) => f.fmt(),
            Self::Boolean(b) => b.fmt(),
            Self::Datetime(dt) => dt.fmt(),
            Self::Array(arr) => arr.fmt(),
            Self::InlineTable(it) => it.fmt(),
        }
    }
}

impl ManifestEditor {
    pub fn from_project(project: &Project) -> anyhow::Result<Self> {
        let manifest_path = project.path.join("Cargo.toml");
        let manifest_content = std::fs::read_to_string(&manifest_path)?;
        let data = manifest_content.parse::<DocumentMut>()?;
        Ok(Self {
            data,
            manifest_path,
        })
    }

    fn get_dep_item(&self, dep_name: &str) -> Option<&Item> {
        self.data
            .get("dependencies")
            .and_then(|item| item.get(dep_name))
    }

    fn get_dep_item_mut(&mut self, dep_name: &str) -> Option<&mut Item> {
        self.data
            .get_mut("dependencies")
            .and_then(|item| item.get_mut(dep_name))
    }

    fn get_dep_format(&self, dep_name: &str) -> Option<DepFormat> {
        let item = self.get_dep_item(dep_name)?;
        match item {
            Item::Value(Value::String(_)) => Some(DepFormat::Simple),
            Item::Table(_) | Item::Value(Value::InlineTable(_)) => Some(DepFormat::Detailed),
            _ => None,
        }
    }

    fn transform_simple_dep_to_detailed(&mut self, dep_name: &str) {
        debug_assert_matches!(self.get_dep_format(dep_name), Some(DepFormat::Simple));

        let item = match self.get_dep_item(dep_name) {
            Some(item) => item.clone(),
            None => return,
        };

        if let Item::Value(Value::String(version)) = item {
            let mut table = toml_edit::InlineTable::new();
            table.insert("version", Value::String(version));
            table.fmt();
            self.data["dependencies"][dep_name] = Item::Value(Value::InlineTable(table));
        }
    }

    fn format_deps(&mut self) {
        if let Some(deps_table) = self
            .data
            .get_mut("dependencies")
            .and_then(|item| item.as_table_mut())
        {
            deps_table.fmt();
        }
    }

    fn simplify_detailed_dep(&mut self, dep_name: &str) {
        let Some(format) = self.get_dep_format(dep_name) else {
            return;
        };

        if matches!(format, DepFormat::Simple) {
            return;
        }

        let Some(item) = self.get_dep_item_mut(dep_name) else {
            return;
        };

        let Some(table) = item.as_table_like_mut() else {
            return;
        };

        if let Some(default_features) = table.get("default-features").and_then(Item::as_bool)
            && default_features
        {
            table.remove("default-features");
        }

        if let Some(features) = table.get("features").and_then(|f| f.as_array())
            && features.is_empty()
        {
            table.remove("features");
        }

        if table.len() == 1
            && let Some(version_item) = table.get("version")
            && let Item::Value(Value::String(version)) = version_item
        {
            *item = Item::Value(Value::String(version.clone()));
        }
    }

    pub fn set_dep_features(
        &mut self,
        dep_name: &str,
        features: Option<Vec<String>>,
        default_features: Option<bool>,
    ) {
        let Some(dep_format) = self.get_dep_format(dep_name) else {
            return;
        };

        if matches!(dep_format, DepFormat::Simple) {
            self.transform_simple_dep_to_detailed(dep_name);
        }

        if let Some(features) = features {
            let features_array = features.into_iter().collect::<toml_edit::Array>();
            self.data["dependencies"][dep_name]["features"] =
                Item::Value(Value::Array(features_array));
        }

        if let Some(default_features) = default_features {
            self.data["dependencies"][dep_name]["default-features"] =
                toml_edit::value(default_features);
        }

        self.simplify_detailed_dep(dep_name);

        self.format_deps();
    }

    pub fn save(&self) -> anyhow::Result<()> {
        let toml = self.to_string();
        std::fs::write("Cargo.test.toml", toml)?;
        Ok(())
    }
}

impl std::fmt::Display for ManifestEditor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.data)
    }
}

#[cfg(test)]
mod test {
    use assert_matches::assert_matches;

    use super::*;

    impl ManifestEditor {
        pub fn new(toml_str: &str) -> anyhow::Result<Self> {
            let data = toml_str.parse::<DocumentMut>()?;
            Ok(Self {
                data,
                manifest_path: PathBuf::new(),
            })
        }
    }

    const INITIAL_MANIFEST: &str = r#"
[dependencies]
test = "2.3"
serde = "1.0"
anyhow = { version = "1.0" }

[dependencies.inquire]
version = "0.3"
features = ["color"]
        "#;

    fn editor() -> ManifestEditor {
        ManifestEditor::new(INITIAL_MANIFEST).unwrap()
    }

    #[test]
    fn initial_formats_and_transform_serde() {
        let mut editor = editor();

        assert_matches!(editor.get_dep_format("test"), Some(DepFormat::Simple));
        assert_matches!(editor.get_dep_format("serde"), Some(DepFormat::Simple));
        assert_matches!(editor.get_dep_format("anyhow"), Some(DepFormat::Detailed));
        assert_matches!(editor.get_dep_format("inquire"), Some(DepFormat::Detailed));

        editor.transform_simple_dep_to_detailed("serde");
        assert_matches!(editor.get_dep_format("serde"), Some(DepFormat::Detailed));

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = "2.3"
serde = { version = "1.0" }
anyhow = { version = "1.0" }

[dependencies.inquire]
version = "0.3"
features = ["color"]
        "#
        );
    }

    #[test]
    fn set_features_on_simple_dep_test() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");

        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"] }
serde = { version = "1.0" }
anyhow = { version = "1.0" }

[dependencies.inquire]
version = "0.3"
features = ["color"]
        "#
        );
    }

    #[test]
    fn simplify_detailed_dep_inquire_with_empty_features() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");
        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);

        editor.set_dep_features("inquire", Some(vec![]), None);

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"] }
serde = { version = "1.0" }
anyhow = { version = "1.0" }
inquire = "0.3"
        "#
        );
    }

    #[test]
    fn set_multiple_features_on_inquire() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");
        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);
        editor.set_dep_features("inquire", Some(vec![]), None); // simplify first
        editor.set_dep_features(
            "inquire",
            Some(vec!["f1".into(), "f2".into(), "f3".into()]),
            None,
        );

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"] }
serde = { version = "1.0" }
anyhow = { version = "1.0" }
inquire = { version = "0.3", features = ["f1", "f2", "f3"] }
        "#
        );
    }

    #[test]
    fn simplify_serde_back_to_simple_value() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");
        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);
        editor.set_dep_features("inquire", Some(vec![]), None);
        editor.set_dep_features(
            "inquire",
            Some(vec!["f1".into(), "f2".into(), "f3".into()]),
            None,
        );

        editor.set_dep_features("serde", None, None);

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"] }
serde = "1.0"
anyhow = { version = "1.0" }
inquire = { version = "0.3", features = ["f1", "f2", "f3"] }
        "#
        );
    }

    #[test]
    fn set_default_features_false_on_test() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");
        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);
        editor.set_dep_features("inquire", Some(vec![]), None);
        editor.set_dep_features(
            "inquire",
            Some(vec!["f1".into(), "f2".into(), "f3".into()]),
            None,
        );
        editor.set_dep_features("serde", None, None);

        editor.set_dep_features("test", None, Some(false));

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"], default-features = false }
serde = "1.0"
anyhow = { version = "1.0" }
inquire = { version = "0.3", features = ["f1", "f2", "f3"] }
        "#
        );
    }

    #[test]
    fn remove_default_features_when_true_on_test() {
        let mut editor = editor();
        editor.transform_simple_dep_to_detailed("serde");
        editor.set_dep_features("test", Some(vec!["derive".into(), "alloc".into()]), None);
        editor.set_dep_features("inquire", Some(vec![]), None);
        editor.set_dep_features(
            "inquire",
            Some(vec!["f1".into(), "f2".into(), "f3".into()]),
            None,
        );
        editor.set_dep_features("serde", None, None);
        editor.set_dep_features("test", None, Some(false));

        editor.set_dep_features("test", None, Some(true));

        assert_eq!(
            editor.to_string(),
            r#"
[dependencies]
test = { version = "2.3", features = ["derive", "alloc"] }
serde = "1.0"
anyhow = { version = "1.0" }
inquire = { version = "0.3", features = ["f1", "f2", "f3"] }
        "#
        );
    }
}
