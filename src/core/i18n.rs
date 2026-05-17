use anyhow::{anyhow, Result};
use fluent::FluentResource;
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use unic_langid::LanguageIdentifier;

pub struct I18n {
    bundles: HashMap<String, fluent::concurrent::FluentBundle<FluentResource>>,
    current_locale: String,
}

impl I18n {
    pub fn from_assets_dir(assets_dir: PathBuf, default_locale: &str) -> Result<Self> {
        let mut bundles = HashMap::new();

        if assets_dir.exists() {
            for entry in fs::read_dir(&assets_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("ftl") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        let source = fs::read_to_string(&path)?;
                        let res = FluentResource::try_new(source)
                            .map_err(|(_res, _errs)| anyhow!("failed to parse FTL"))?;

                        let langid: LanguageIdentifier = file_stem
                            .parse()
                            .map_err(|_| anyhow!("invalid locale name: {}", file_stem))?;

                        let mut bundle: fluent::concurrent::FluentBundle<FluentResource> =
                            fluent::concurrent::FluentBundle::new_concurrent(vec![langid]);

                        bundle
                            .add_resource(res)
                            .map_err(|_e| anyhow!("failed to add FTL resource for {file_stem}"))?;
                        bundles.insert(file_stem.to_string(), bundle);
                    }
                }
            }
        }

        Ok(Self {
            bundles,
            current_locale: default_locale.to_string(),
        })
    }

    pub fn empty(default_locale: &str) -> Self {
        Self {
            bundles: HashMap::new(),
            current_locale: default_locale.to_string(),
        }
    }

    pub fn set_locale(&mut self, locale: &str) {
        if self.bundles.contains_key(locale) {
            self.current_locale = locale.to_string();
        }
    }

    pub fn locale(&self) -> &str {
        &self.current_locale
    }

    pub fn tr(&self, key: &str) -> String {
        let Some(bundle): Option<&fluent::concurrent::FluentBundle<FluentResource>> =
            self.bundles.get(&self.current_locale)
        else {
            return key.to_string();
        };
        let msg = match bundle.get_message(key) {
            Some(m) => m,
            None => return key.to_string(),
        };
        let value = match msg.value() {
            Some(v) => v,
            None => return key.to_string(),
        };
        let mut errors = vec![];
        let pattern = bundle.format_pattern(value, None, &mut errors);
        pattern.to_string()
    }
}
