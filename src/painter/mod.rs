use eframe::egui;

/// Painter 用ポイント (マップ座標系)。
#[derive(Debug, Clone)]
pub struct PainterPoint {
    pub x: f32,
    pub y: f32,
}

/// Painter のグリッド/ポイントモード。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GridMode {
    None,
    VoronoiPoints,
    HexGrid,
}

/// Hex グリッド設定。
#[derive(Debug, Clone)]
pub struct HexGridConfig {
    /// 1セルの一辺の長さ（ピクセル単位）
    pub cell_size: f32,
    /// グリッド原点（マップ座標系）
    pub origin: egui::Pos2,
}

impl Default for HexGridConfig {
    fn default() -> Self {
        Self {
            cell_size: 64.0,
            origin: egui::Pos2::new(0.0, 0.0),
        }
    }
}

/// マップ座標を Hex セル座標に変換する（軸平行「ポイントトップ」六角形グリッド）。
pub fn map_pos_to_hex_cell(map_pos: egui::Pos2, config: &HexGridConfig) -> (i32, i32) {
    let q = (map_pos.x - config.origin.x) / (config.cell_size * 3f32.sqrt() / 2.0);
    let r = (map_pos.y - config.origin.y) / config.cell_size;
    (q.round() as i32, r.round() as i32)
}

/// Hex セルを6頂点の多角形に変換する（マップ座標系）。
pub fn hex_cell_to_polygon(cell: (i32, i32), config: &HexGridConfig) -> [egui::Pos2; 6] {
    let (q, r) = cell;
    let fq = q as f32;
    let fr = r as f32;

    // セル中心座標（平行六角形近似）
    let cx = config.origin.x + fq * (config.cell_size * 3f32.sqrt() / 2.0);
    let cy = config.origin.y + fr * config.cell_size;

    let mut points = [egui::Pos2::ZERO; 6];
    for i in 0..6 {
        // 0〜5 の各頂点に対して角度 60 度刻み
        let angle = (60.0 * i as f32).to_radians();
        let x = cx + config.cell_size * angle.cos();
        let y = cy + config.cell_size * angle.sin();
        points[i] = egui::Pos2::new(x, y);
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_config_default_has_reasonable_values() {
        let cfg = HexGridConfig::default();
        assert!(cfg.cell_size > 0.0);
    }
}
