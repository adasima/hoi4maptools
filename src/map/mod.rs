pub mod command;
pub mod definition;
pub mod graph;

use crate::map::definition::DefinitionTable;
use crate::map::graph::ProvinceGraph;
use std::path::PathBuf;

use eframe::egui;

/// プロジェクトの状態 (読み込まれたマップデータ)
pub struct ProjectState {
    /// 生のピクセルデータ (RGB)
    pub pixels: Vec<u8>,
    pub width: u32,
    pub height: u32,
    /// プロヴィンス定義
    pub definitions: DefinitionTable,
    /// プロヴィンスグラフ
    pub graph: ProvinceGraph,
    /// ファイルのロード元パス
    pub project_dir: PathBuf,
    /// 変更された領域を追跡し、テクスチャの部分更新に使用する
    pub dirty_rect: Option<egui::Rect>,
}
