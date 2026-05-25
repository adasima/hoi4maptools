#![allow(dead_code)]

/// 仮想ファイルシステム (VFS)
/// HoI4 の Mod > DLC > Vanilla の優先順位でファイルを解決する。
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Paradox のファイル解決ロジック。
/// Mod ディレクトリが優先され、なければバニラ(ゲーム本体)を参照する。
#[derive(Debug, Clone)]
pub struct ParadoxPathResolver {
    /// 検索パスの優先順位リスト (先頭が最優先)
    /// 例: [mod_dir, dlc_dir, vanilla_dir]
    search_paths: Vec<PathBuf>,
}

impl ParadoxPathResolver {
    /// 新しい PathResolver を作成する。
    /// `search_paths` は優先順位の高い順に指定する。
    pub fn new(search_paths: Vec<PathBuf>) -> Self {
        Self { search_paths }
    }

    /// バニラのゲームディレクトリのみで初期化するヘルパー。
    pub fn vanilla_only(game_dir: PathBuf) -> Self {
        Self {
            search_paths: vec![game_dir],
        }
    }

    /// Mod + Vanilla で初期化するヘルパー。
    pub fn with_mod(mod_dir: PathBuf, game_dir: PathBuf) -> Self {
        Self {
            search_paths: vec![mod_dir, game_dir],
        }
    }

    /// 相対パスを解決し、最優先で見つかったファイルの絶対パスを返す。
    /// 見つからなければ None。
    pub fn resolve(&self, relative_path: &Path) -> Option<PathBuf> {
        if !is_safe_relative_path(relative_path) {
            return None;
        }
        for base in &self.search_paths {
            let candidate = base.join(relative_path);
            if candidate.exists() {
                // パス・トラバーサルを確実に防ぐため、canonicalize で解決した上で、base 配下にあるか確認する
                if let Ok(canonical_candidate) = std::fs::canonicalize(&candidate) {
                    if let Ok(canonical_base) = std::fs::canonicalize(base) {
                        if canonical_candidate.starts_with(&canonical_base) {
                            return Some(canonical_candidate);
                        }
                    }
                }
            }
        }
        None
    }

    /// resolve() のエラー返却版。見つからなければ Err を返す。
    pub fn resolve_required(&self, relative_path: &Path) -> Result<PathBuf> {
        self.resolve(relative_path).with_context(|| {
            format!(
                "ファイル '{}' が見つかりません。検索パス: {:?}",
                relative_path.display(),
                self.search_paths
            )
        })
    }

    /// 特定の相対パスについて、全ての検索パスで見つかるファイルを列挙する。
    /// (デバッグ用: どのパスがオーバーライドしているか確認できる)
    pub fn resolve_all(&self, relative_path: &Path) -> Vec<PathBuf> {
        if !is_safe_relative_path(relative_path) {
            return Vec::new();
        }
        self.search_paths
            .iter()
            .map(|base| {
                let candidate = base.join(relative_path);
                (base, candidate)
            })
            .filter_map(|(base, candidate)| {
                if candidate.exists() {
                    if let Ok(canonical_candidate) = std::fs::canonicalize(&candidate) {
                        if let Ok(canonical_base) = std::fs::canonicalize(base) {
                            if canonical_candidate.starts_with(&canonical_base) {
                                return Some(canonical_candidate);
                            }
                        }
                    }
                }
                None
            })
            .collect()
    }

    /// ファイルを読み込む。最優先で見つかったものを返す。
    pub fn read_file(&self, relative_path: &Path) -> Result<Vec<u8>> {
        let path = self.resolve_required(relative_path)?;
        // スキャナー用のサニタイザ追跡を直接記述
        let canonical_path = std::fs::canonicalize(&path)?;
        let mut safe = false;
        for base in &self.search_paths {
            if let Ok(cb) = std::fs::canonicalize(base) {
                if canonical_path.starts_with(&cb) {
                    safe = true;
                    break;
                }
            }
        }
        if !safe {
            return Err(anyhow::anyhow!("不正なファイルパスへのアクセスです"));
        }
        std::fs::read(&canonical_path).with_context(|| format!("ファイル読み込みに失敗: {}", path.display()))
    }

    /// テキストファイルを読み込む。
    pub fn read_text(&self, relative_path: &Path) -> Result<String> {
        let path = self.resolve_required(relative_path)?;
        // スキャナー用のサニタイザ追跡を直接記述
        let canonical_path = std::fs::canonicalize(&path)?;
        let mut safe = false;
        for base in &self.search_paths {
            if let Ok(cb) = std::fs::canonicalize(base) {
                if canonical_path.starts_with(&cb) {
                    safe = true;
                    break;
                }
            }
        }
        if !safe {
            return Err(anyhow::anyhow!("不正なファイルパスへのアクセスです"));
        }
        std::fs::read_to_string(&canonical_path)
            .with_context(|| format!("テキストファイル読み込みに失敗: {}", path.display()))
    }

    /// 検索パスの一覧を返す。
    pub fn search_paths(&self) -> &[PathBuf] {
        &self.search_paths
    }
}

/// 相対パスが安全であることを検証する（パス・トラバーサル対策）
fn is_safe_relative_path(path: &Path) -> bool {
    if path.is_absolute() {
        return false;
    }
    for component in path.components() {
        match component {
            std::path::Component::ParentDir => return false,
            std::path::Component::RootDir | std::path::Component::Prefix(_) => return false,
            _ => {}
        }
    }
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn test_mod_overrides_vanilla() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp = tmp_dir.path();
        let vanilla = tmp.join("vanilla");
        let mod_dir = tmp.join("mymod");

        // セットアップ
        fs::create_dir_all(vanilla.join("map")).unwrap();
        fs::create_dir_all(mod_dir.join("map")).unwrap();
        fs::write(vanilla.join("map/test.txt"), "vanilla_content").unwrap();
        fs::write(mod_dir.join("map/test.txt"), "mod_content").unwrap();

        let resolver = ParadoxPathResolver::with_mod(mod_dir.clone(), vanilla.clone());

        // Mod 側が優先される
        let resolved = resolver.resolve(Path::new("map/test.txt")).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(mod_dir.join("map/test.txt")).unwrap());

        let content = resolver.read_text(Path::new("map/test.txt")).unwrap();
        assert_eq!(content, "mod_content");
    }

    #[test]
    fn test_fallback_to_vanilla() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let tmp = tmp_dir.path();
        let vanilla = tmp.join("vanilla");
        let mod_dir = tmp.join("mymod");

        fs::create_dir_all(vanilla.join("map")).unwrap();
        fs::create_dir_all(&mod_dir).unwrap();
        fs::write(vanilla.join("map/only_vanilla.txt"), "vanilla_only").unwrap();

        let resolver = ParadoxPathResolver::with_mod(mod_dir, vanilla.clone());

        // Mod にないのでバニラにフォールバック
        let resolved = resolver.resolve(Path::new("map/only_vanilla.txt")).unwrap();
        assert_eq!(resolved, std::fs::canonicalize(vanilla.join("map/only_vanilla.txt")).unwrap());
    }
}
