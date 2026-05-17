/// definition.csv の解析と管理。
/// HoI4 の definition.csv フォーマット:
///   provinceId;r;g;b;type;isCoastal;terrain;continent
/// 例: 1;128;0;64;land;false;plains;1

use anyhow::{Context, Result};
use std::collections::HashMap;
use std::path::Path;

use super::graph::{ProvinceColor, ProvinceId};

/// プロヴィンスの種別。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProvinceType {
    Land,
    Sea,
    Lake,
    Unknown(String),
}

impl ProvinceType {
    pub fn from_str(s: &str) -> Self {
        match s.trim().to_lowercase().as_str() {
            "land" => Self::Land,
            "sea" => Self::Sea,
            "lake" => Self::Lake,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Land => "land",
            Self::Sea => "sea",
            Self::Lake => "lake",
            Self::Unknown(s) => s.as_str(),
        }
    }
}

/// definition.csv の1行分のデータ。
#[derive(Debug, Clone)]
pub struct ProvinceDefinition {
    pub id: ProvinceId,
    pub color: ProvinceColor,
    pub province_type: ProvinceType,
    pub is_coastal: bool,
    pub terrain: String,
    pub continent: u32,
    /// 生の行データ (書き出し時にフォーマットを維持するため)
    pub raw_extra_fields: Vec<String>,
}

/// definition.csv を管理する構造体。
#[derive(Debug, Clone)]
pub struct DefinitionTable {
    /// ID順のエントリ
    entries: Vec<ProvinceDefinition>,
    /// RGB色キー -> ID のルックアップ
    color_to_id: HashMap<u32, ProvinceId>,
    /// ファイル先頭のコメント/ヘッダー行を保持 (書き出し時に維持)
    header_lines: Vec<String>,
}

impl DefinitionTable {
    /// 空のテーブルを作成する。
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            color_to_id: HashMap::new(),
            header_lines: Vec::new(),
        }
    }

    /// definition.csv ファイルを解析する。
    /// HoI4の definition.csv はセミコロン区切り。
    pub fn load_from_file(path: &Path) -> Result<Self> {
        let content = std::fs::read_to_string(path)
            .with_context(|| format!("definition.csv の読み込みに失敗: {}", path.display()))?;

        Self::parse(&content)
    }

    /// 文字列から解析する。
    pub fn parse(content: &str) -> Result<Self> {
        let mut entries = Vec::new();
        let mut color_to_id = HashMap::new();
        let mut header_lines = Vec::new();
        let mut found_data = false;

        for (line_no, line) in content.lines().enumerate() {
            let trimmed = line.trim();

            // 空行やコメント行
            if trimmed.is_empty() || trimmed.starts_with('#') {
                if !found_data {
                    header_lines.push(line.to_string());
                }
                continue;
            }

            let parts: Vec<&str> = line.split(';').collect();
            if parts.len() < 5 {
                // ヘッダー行や不完全な行はスキップ
                if !found_data {
                    header_lines.push(line.to_string());
                }
                continue;
            }

            // ID のパース
            let id: ProvinceId = match parts[0].trim().parse() {
                Ok(v) => v,
                Err(_) => {
                    if !found_data {
                        header_lines.push(line.to_string());
                    }
                    continue;
                }
            };

            found_data = true;

            // RGB
            let r: u8 = parts[1].trim().parse().with_context(|| {
                format!("行 {}: R値の解析に失敗", line_no + 1)
            })?;
            let g: u8 = parts[2].trim().parse().with_context(|| {
                format!("行 {}: G値の解析に失敗", line_no + 1)
            })?;
            let b: u8 = parts[3].trim().parse().with_context(|| {
                format!("行 {}: B値の解析に失敗", line_no + 1)
            })?;

            let color = ProvinceColor::new(r, g, b);
            let province_type = ProvinceType::from_str(parts.get(4).unwrap_or(&"land"));
            let is_coastal = parts.get(5).map_or(false, |s| {
                s.trim().to_lowercase() == "true"
            });
            let terrain = parts.get(6).unwrap_or(&"unknown").trim().to_string();
            let continent: u32 = parts.get(7).and_then(|s| s.trim().parse().ok()).unwrap_or(0);

            // 残りのフィールドを保持
            let raw_extra_fields: Vec<String> = parts[8..].iter().map(|s| s.to_string()).collect();

            let def = ProvinceDefinition {
                id,
                color,
                province_type,
                is_coastal,
                terrain,
                continent,
                raw_extra_fields,
            };

            color_to_id.insert(color.to_key(), id);
            entries.push(def);
        }

        log::info!("definition.csv: {}個のプロヴィンスを読み込み", entries.len());

        Ok(Self {
            entries,
            color_to_id,
            header_lines,
        })
    }

    /// definition.csv として書き出す。ヘッダーとフォーマットを維持する。
    pub fn write_to_file(&self, path: &Path) -> Result<()> {
        let mut output = String::new();

        // ヘッダー行を復元
        for line in &self.header_lines {
            output.push_str(line);
            output.push('\n');
        }

        // データ行
        for entry in &self.entries {
            let mut line = format!(
                "{};{};{};{};{};{};{};{}",
                entry.id,
                entry.color.r,
                entry.color.g,
                entry.color.b,
                entry.province_type.as_str(),
                if entry.is_coastal { "true" } else { "false" },
                entry.terrain,
                entry.continent,
            );

            // 追加フィールドがあれば付加
            for extra in &entry.raw_extra_fields {
                line.push(';');
                line.push_str(extra);
            }

            output.push_str(&line);
            output.push('\n');
        }

        std::fs::write(path, &output)
            .with_context(|| format!("definition.csv の書き出しに失敗: {}", path.display()))?;

        log::info!("definition.csv: {}個のプロヴィンスを書き出し", self.entries.len());
        Ok(())
    }

    /// RGB色 -> プロヴィンスID のマッピングを返す。
    /// ProvinceGraph 構築時に使用する。
    pub fn color_id_map(&self) -> &HashMap<u32, ProvinceId> {
        &self.color_to_id
    }

    /// IDで検索。

    pub fn get_mut(&mut self, id: ProvinceId) -> Option<&mut ProvinceDefinition> {
        self.entries.iter_mut().find(|def| def.id == id)
    }

    pub fn get(&self, id: ProvinceId) -> Option<&ProvinceDefinition> {
        self.entries.iter().find(|e| e.id == id)
    }

    /// 全エントリのイテレータ。
    pub fn entries(&self) -> &[ProvinceDefinition] {
        &self.entries
    }

    /// 使用中のRGB色キーの集合を返す。
        pub fn get_color_map(&self) -> &std::collections::HashMap<u32, ProvinceId> {
        &self.color_to_id
    }

    pub fn used_colors(&self) -> std::collections::HashSet<u32> {
        self.color_to_id.keys().copied().collect()
    }

    /// 未使用の RGB カラーを1つ見つけて返す。
    /// provinces.bmp 上でユニークな色が必要なため、衝突しない色を生成する。
    pub fn allocate_unused_color(&self) -> Option<ProvinceColor> {
        let used = self.used_colors();
        // 系統的に検索 (0,0,0 は避ける ← HoI4では黒が特殊な意味を持つ場合がある)
        for r in 1u8..=255 {
            for g in 0u8..=255 {
                for b in 0u8..=255 {
                    let key = (r as u32) << 16 | (g as u32) << 8 | b as u32;
                    if !used.contains(&key) {
                        return Some(ProvinceColor::new(r, g, b));
                    }
                }
            }
        }
        None
    }

    /// 新しいプロヴィンスを追加する。IDは既存の最大値+1。
    pub fn add_province(
        &mut self,
        color: ProvinceColor,
        province_type: ProvinceType,
        terrain: &str,
        continent: u32,
    ) -> ProvinceId {
        let next_id = self.entries.iter().map(|e| e.id).max().unwrap_or(0) + 1;

        let def = ProvinceDefinition {
            id: next_id,
            color,
            province_type,
            is_coastal: false,
            terrain: terrain.to_string(),
            continent,
            raw_extra_fields: Vec::new(),
        };

        self.color_to_id.insert(color.to_key(), next_id);
        self.entries.push(def);
        next_id
    }

    /// IDを指定してプロヴィンス定義を削除する。(Undo用)
    pub fn remove(&mut self, id: ProvinceId) {
        if let Some(pos) = self.entries.iter().position(|e| e.id == id) {
            let entry = self.entries.remove(pos);
            self.color_to_id.remove(&entry.color.to_key());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_definition_csv() {
        let csv = r#"province;red;green;blue;type;coastal;terrain;continent
1;128;0;64;land;false;plains;1
2;0;128;255;sea;false;ocean;0
3;64;64;64;lake;false;lakes;1
"#;
        let table = DefinitionTable::parse(csv).unwrap();
        assert_eq!(table.entries().len(), 3);
        assert_eq!(table.get(1).unwrap().terrain, "plains");
        assert_eq!(table.get(2).unwrap().province_type, ProvinceType::Sea);
    }

    #[test]
    fn test_allocate_unused_color() {
        let table = DefinitionTable::new();
        let color = table.allocate_unused_color().unwrap();
        // (0,0,0) は避けるので r >= 1 のはず
        assert!(color.r >= 1 || color.g >= 1 || color.b >= 1);
    }

    #[test]
    fn test_roundtrip_write_read() {
        let csv = "1;100;200;50;land;true;forest;2\n";
        let table = DefinitionTable::parse(csv).unwrap();

        let tmp_path = std::env::temp_dir().join("ws_def_test.csv");
        table.write_to_file(&tmp_path).unwrap();

        let table2 = DefinitionTable::load_from_file(&tmp_path).unwrap();
        assert_eq!(table2.entries().len(), 1);
        assert_eq!(table2.get(1).unwrap().color.r, 100);

        let _ = std::fs::remove_file(&tmp_path);
    }
}
