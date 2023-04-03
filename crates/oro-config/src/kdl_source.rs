use config::{ConfigError, FileStoredFormat, Format, Map, Source, Value, ValueKind};
use kdl::{KdlDocument, KdlValue};

#[derive(Clone, Debug)]
pub(crate) struct KdlSource(KdlDocument);

impl Source for KdlSource {
    fn clone_into_box(&self) -> Box<dyn Source + Send + Sync> {
        Box::new(self.clone())
    }

    fn collect(&self) -> Result<Map<String, Value>, ConfigError> {
        let mut map = Map::new();
        for node in self.0.nodes() {
            let key = node.name().to_string();
            if let Some(value) = node.get(0) {
                let value = Value::new(
                    Some(&if let Some(str) = value.as_string() {
                        str.to_owned()
                    } else {
                        value.to_string()
                    }),
                    value_kind(value),
                );
                map.insert(key, value);
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
