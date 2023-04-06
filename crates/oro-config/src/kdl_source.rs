use config::{ConfigError, FileStoredFormat, Format, Map, Source, Value, ValueKind};
use kdl::{KdlDocument, KdlNode, KdlValue};

#[derive(Clone, Debug)]
pub(crate) struct KdlSource(KdlDocument);

impl Source for KdlSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let mut map = Map::new();
        if let Some(config_node) = self.0.get("options") {
            if let Some(children) = config_node.children() {
                for node in children.nodes() {
                    map.insert(node.name().value().to_string(), node_value(node));
                }
            }
        }
        Ok(map)
    }
}

#[derive(Clone, Debug)]
pub(crate) struct KdlFormat;

impl Format for KdlFormat {
    fn parse(
        &self,
        _uri: Option<&String>,
        text: &str,
    ) -> Result<Map<String, Value>, Box<dyn std::error::Error + Send + Sync>> {
        Ok(KdlSource(text.parse()?).collect()?)
    }
}

impl FileStoredFormat for KdlFormat {
    fn file_extensions(&self) -> &'static [&'static str] {
        &["kdl"]
    }
}

fn value_kind(value: &KdlValue) -> ValueKind {
    if let Some(str) = value.as_string() {
        ValueKind::String(str.into())
    } else if let Some(num) = value.as_i64() {
        ValueKind::I64(num)
    } else if let Some(float) = value.as_f64() {
        ValueKind::Float(float)
    } else if let Some(boolean) = value.as_bool() {
        ValueKind::Boolean(boolean)
    } else {
        ValueKind::Nil
    }
}

fn map_kind(value: impl Iterator<Item = (String, Value)>) -> ValueKind {
    ValueKind::Table(value.collect())
}

fn array_kind(value: impl Iterator<Item = Value>) -> ValueKind {
    ValueKind::Array(value.collect())
}

fn node_value(node: &KdlNode) -> Value {
    let mut entries = node.entries().iter().filter(|e| e.name().is_some());
    let len = entries.clone().count();
    // foo 1 => { foo: 1 }
    //
    // Technically, this could semantically be an array as well, but we choose
    // to treat single-entries as single values.
    if len == 1 {
        Value::new(
            None,
            value_kind(entries.next().expect("checked length already").value()),
        )
    // foo 1 2 3 => { foo: [1, 2, 3] }
    } else if len > 1 {
        Value::new(
            None,
            array_kind(entries.map(|e| Value::new(None, value_kind(e.value())))),
        )
    } else if let Some(children) = node.children() {
        let dash_children = children
            .nodes()
            .iter()
            .all(|node| node.name().value() == "-");
        if dash_children {
            // foo {
            //   - 1
            //   - 2
            //   - {
            //     bar 3
            //   }
            // }
            // => { foo: [1, 2, { bar: 3 }] }
            Value::new(None, array_kind(children.nodes().iter().map(node_value)))
        } else {
            // foo {
            //     bar {
            //         baz 1
            //     }
            // }
            // => { foo: { bar: { baz: 1 } } }
            Value::new(
                None,
                map_kind(
                    children
                        .nodes()
                        .iter()
                        .map(|node| (node.name().value().to_string(), node_value(node))),
                ),
            )
        }
    } else {
        Value::new(None, ValueKind::Nil)
    }
}
