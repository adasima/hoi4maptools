#![allow(dead_code)]
use eframe::egui;

/// レンダラーモジュール。
/// Dirty Rect トラッキングとレイヤー合成を管理する。
/// 変更された矩形領域を追跡し、テクスチャの部分更新を可能にする。
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct DirtyRect {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

/// Dirty Rect のトラッカー。
/// 複数の変更をマージして最小の更新矩形を計算する。
pub struct DirtyRectTracker {
    /// 現在の dirty 領域 (None = クリーン)
    current: Option<DirtyRect>,
}

#[allow(dead_code)]
impl DirtyRectTracker {
    pub fn new() -> Self {
        Self { current: None }
    }

    /// 変更された領域をマークする。
    pub fn mark_dirty(&mut self, x: u32, y: u32, width: u32, height: u32) {
        match &mut self.current {
            None => {
                self.current = Some(DirtyRect {
                    x,
                    y,
                    width,
                    height,
                });
            }
            Some(rect) => {
                // 既存のrectとマージ (バウンディングボックスの結合)
                let min_x = rect.x.min(x);
                let min_y = rect.y.min(y);
                let max_x = (rect.x + rect.width).max(x + width);
                let max_y = (rect.y + rect.height).max(y + height);
                rect.x = min_x;
                rect.y = min_y;
                rect.width = max_x - min_x;
                rect.height = max_y - min_y;
            }
        }
    }

    /// 全体をdirtyにする (初回ロード時など)。
    pub fn mark_all_dirty(&mut self, width: u32, height: u32) {
        self.current = Some(DirtyRect {
            x: 0,
            y: 0,
            width,
            height,
        });
    }

    /// 現在の dirty rect を取得し、クリーンにリセットする。
    pub fn take_dirty(&mut self) -> Option<DirtyRect> {
        self.current.take()
    }

    /// dirty な領域があるか。
    pub fn is_dirty(&self) -> bool {
        self.current.is_some()
    }
}

/// レイヤーの種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[allow(dead_code)]
pub enum LayerType {
    Provinces,
    Terrain,
    Rivers,
    Trees,
    Heightmap,
    /// 半透明のユーザー参照画像
    Reference,
}

/// 1枚のレイヤー情報。
#[allow(dead_code)]
pub struct MapLayer {
    pub layer_type: LayerType,
    pub visible: bool,
    pub opacity: f32, // 0.0 ~ 1.0
    /// テクスチャハンドル (egui::TextureHandle に関連付けされる)
    pub texture_handle: Option<egui::TextureHandle>,
}

#[allow(dead_code)]
impl MapLayer {
    pub fn new(layer_type: LayerType) -> Self {
        Self {
            layer_type,
            visible: true,
            opacity: 1.0,
            texture_handle: None,
        }
    }
}

/// マップビューポートの状態。
pub struct MapViewport {
    /// パンオフセット (ピクセル単位)
    pub offset: egui::Vec2,
    /// ズーム倍率
    pub zoom: f32,
    /// レイヤーリスト
    #[allow(dead_code)]
    pub layers: Vec<MapLayer>,
    /// dirty rect トラッカー
    pub dirty_tracker: DirtyRectTracker,
}

impl MapViewport {
    pub fn new() -> Self {
        Self {
            offset: egui::Vec2::ZERO,
            zoom: 1.0,
            layers: Vec::new(),
            dirty_tracker: DirtyRectTracker::new(),
        }
    }

    /// ズームイン。
    pub fn zoom_in(&mut self) {
        self.zoom = (self.zoom * 1.1).min(50.0);
    }

    /// ズームアウト。
    pub fn zoom_out(&mut self) {
        self.zoom = (self.zoom / 1.1).max(0.01);
    }

    /// 画面座標をマップ座標に変換する。
    pub fn screen_to_map(&self, screen_pos: egui::Pos2, viewport_rect: egui::Rect) -> egui::Pos2 {
        let center = viewport_rect.center();
        let relative = screen_pos - center;
        egui::Pos2::new(
            (relative.x / self.zoom) - self.offset.x,
            (relative.y / self.zoom) - self.offset.y,
        )
    }

    /// マップ座標を画面座標に変換する。
    pub fn map_to_screen(&self, map_pos: egui::Pos2, viewport_rect: egui::Rect) -> egui::Pos2 {
        let center = viewport_rect.center();
        egui::Pos2::new(
            (map_pos.x + self.offset.x) * self.zoom + center.x,
            (map_pos.y + self.offset.y) * self.zoom + center.y,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dirty_rect_merge() {
        let mut tracker = DirtyRectTracker::new();
        tracker.mark_dirty(10, 10, 20, 20);
        tracker.mark_dirty(50, 50, 10, 10);

        let rect = tracker.take_dirty().unwrap();
        // マージされたバウンディングボックス
        assert_eq!(rect.x, 10);
        assert_eq!(rect.y, 10);
        assert_eq!(rect.width, 50); // 50+10 - 10
        assert_eq!(rect.height, 50);

        // 取得後はクリーン
        assert!(!tracker.is_dirty());
    }

    #[test]
    fn test_coordinate_transforms() {
        let mut viewport = MapViewport::new();
        viewport.zoom = 2.0;
        viewport.offset = egui::Vec2::new(10.0, 20.0);

        let viewport_rect =
            egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(800.0, 600.0));

        // Center is (400, 300)

        // Test map_to_screen
        // Map pos: (100, 50)
        // Screen X = (100 + 10) * 2 + 400 = 110 * 2 + 400 = 620
        // Screen Y = (50 + 20) * 2 + 300 = 70 * 2 + 300 = 440
        let map_pos = egui::pos2(100.0, 50.0);
        let screen_pos = viewport.map_to_screen(map_pos, viewport_rect);
        assert_eq!(screen_pos.x, 620.0);
        assert_eq!(screen_pos.y, 440.0);

        // Test screen_to_map
        let converted_map_pos = viewport.screen_to_map(screen_pos, viewport_rect);
        assert_eq!(converted_map_pos.x, 100.0);
        assert_eq!(converted_map_pos.y, 50.0);

        // Test roundtrip
        let original_screen_pos = egui::pos2(123.0, 456.0);
        let map_pos2 = viewport.screen_to_map(original_screen_pos, viewport_rect);
        let screen_pos2 = viewport.map_to_screen(map_pos2, viewport_rect);
        assert!((original_screen_pos.x - screen_pos2.x).abs() < 1e-4);
        assert!((original_screen_pos.y - screen_pos2.y).abs() < 1e-4);
    }
}
