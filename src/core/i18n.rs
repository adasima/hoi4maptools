#![allow(dead_code)]
use anyhow::Result;
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

        // パス・トラバーサル対策のバリデーション
        // 入力パスの文字列に親ディレクトリ参照が含まれていないか厳密にチェック
        let assets_dir_str = assets_dir.to_string_lossy();
        if assets_dir_str.contains("..") {
            return Err(anyhow::anyhow!("無効なアセットディレクトリパス（親ディレクトリへの参照が含まれています）"));
        }

        // 各パスコンポーネントを検証
        for component in assets_dir.components() {
            if let std::path::Component::ParentDir = component {
                return Err(anyhow::anyhow!("無効なアセットディレクトリパス（パス・トラバーサルの疑い）"));
            }
        }

        // パスが存在する場合のみ canonicalize() を行い、存在しない場合は即座に早期リターンする
        let safe_assets_dir = if assets_dir.exists() {
            fs::canonicalize(&assets_dir)
                .map_err(|e| anyhow::anyhow!("アセットディレクトリの正規化に失敗しました: {}", e))?
        } else {
            return Ok(Self::empty(default_locale));
        };

        for component in safe_assets_dir.components() {
            if let std::path::Component::ParentDir = component {
                return Err(anyhow::anyhow!("無効なアセットディレクトリパス（パス・トラバーサルの疑い）"));
            }
        }

        if safe_assets_dir.exists() {
            // read_dir を呼び出す直前に、安全のために再度 canonicalize して得られたパスを直接渡す
            let canonical_dir = fs::canonicalize(&safe_assets_dir)?;
            for entry in fs::read_dir(&canonical_dir)? {
                let entry = entry?;
                let path = entry.path();
                if path.extension().and_then(|s| s.to_str()) == Some("ftl") {
                    if let Some(file_stem) = path.file_stem().and_then(|s| s.to_str()) {
                        // ターゲットパスの正規化
                        let canonical_path = fs::canonicalize(&path)?;
                        // safe_assets_dir 配下であることを検証
                        if !canonical_path.starts_with(&safe_assets_dir) {
                            return Err(anyhow::anyhow!("不正なパスへのアクセスを検出しました"));
                        }

                        let source = fs::read_to_string(&canonical_path)?;
                        let res = FluentResource::try_new(source)
                            .map_err(|(_res, _errs)| anyhow::anyhow!("failed to parse FTL"))?;

                        let langid: LanguageIdentifier = file_stem
                            .parse()
                            .map_err(|_| anyhow::anyhow!("invalid locale name: {}", file_stem))?;

                        let mut bundle: fluent::concurrent::FluentBundle<FluentResource> =
                            fluent::concurrent::FluentBundle::new_concurrent(vec![langid]);

                        bundle
                            .add_resource(res)
                            .map_err(|_e| anyhow::anyhow!("failed to add FTL resource for {file_stem}"))?;
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
