/// プロヴィンスのグラフ構造と空間インデックス。
/// 隣接情報の管理と、座標からプロヴィンスIDへの高速ルックアップを提供する。
use rstar::{PointDistance, RTree, RTreeObject, AABB};
use std::collections::{HashMap, HashSet};

/// プロヴィンスID (HoI4 の definition.csv における ID)
pub type ProvinceId = u32;

/// RGB カラー値 (provinces.bmp のピクセル色)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ProvinceColor {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

impl ProvinceColor {
    pub fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    /// RGB値を u32 のキーに変換する (ルックアップテーブル用)。
    pub fn to_key(&self) -> u32 {
        (self.r as u32) << 16 | (self.g as u32) << 8 | self.b as u32
    }

    /// u32 キーから ProvinceColor に戻す。
    pub fn from_key(key: u32) -> Self {
        Self {
            r: ((key >> 16) & 0xFF) as u8,
            g: ((key >> 8) & 0xFF) as u8,
            b: (key & 0xFF) as u8,
        }
    }
}

/// プロヴィンスの基本データ。
#[derive(Debug, Clone)]
pub struct ProvinceData {
    pub id: ProvinceId,
    pub color: ProvinceColor,
    /// プロヴィンスに属するピクセル数 (面積の指標)
    pub pixel_count: u32,
    /// 座標の合計 (重心計算用)
    pub sum_x: u64,
    pub sum_y: u64,
    /// 重心座標
    pub centroid: [f64; 2],
    /// バウンディングボックス [min_x, min_y, max_x, max_y]
    pub bounds: [u32; 4],
}

impl ProvinceData {
    /// 重心座標を (x, y) で返す。
    pub fn centroid(&self) -> (f32, f32) {
        (self.centroid[0] as f32, self.centroid[1] as f32)
    }
}

/// R*木用のポイントエントリ。
/// プロヴィンスの重心をインデックスし、最近傍検索に使う。
#[derive(Debug, Clone)]
pub struct ProvinceSpatialEntry {
    pub id: ProvinceId,
    pub centroid: [f64; 2],
}

impl RTreeObject for ProvinceSpatialEntry {
    type Envelope = AABB<[f64; 2]>;

    fn envelope(&self) -> Self::Envelope {
        AABB::from_point(self.centroid)
    }
}

impl PointDistance for ProvinceSpatialEntry {
    fn distance_2(&self, point: &[f64; 2]) -> f64 {
        let dx = self.centroid[0] - point[0];
        let dy = self.centroid[1] - point[1];
        dx * dx + dy * dy
    }
}

/// プロヴィンスの隣接関係とデータを管理するグラフ。
pub struct ProvinceGraph {
    /// 全プロヴィンスデータ (ID -> Data)
    provinces: HashMap<ProvinceId, ProvinceData>,
    /// 隣接リスト (ID -> 隣接IDの集合)
    adjacency: HashMap<ProvinceId, HashSet<ProvinceId>>,
    /// RGB色 -> プロヴィンスID の高速ルックアップ (u32 key)
    color_to_id: HashMap<u32, ProvinceId>,
    /// R*木による空間インデックス (重心ベース)
    spatial_index: RTree<ProvinceSpatialEntry>,
}

impl ProvinceGraph {
    /// 空のグラフを作成する。
    pub fn new() -> Self {
        Self {
            provinces: HashMap::new(),
            adjacency: HashMap::new(),
            color_to_id: HashMap::new(),
            spatial_index: RTree::new(),
        }
    }

    /// provinces.bmp の生ピクセルデータからグラフを構築する。
    /// 隣接関係は4近傍（上下左右）で判定する。
    pub fn build_from_pixels(
        pixels: &[u8], // RGB, 3 bytes per pixel
        width: u32,
        height: u32,
        color_id_map: &HashMap<u32, ProvinceId>,
    ) -> Self {
        let mut provinces: HashMap<ProvinceId, ProvinceData> = HashMap::new();
        let mut adjacency: HashMap<ProvinceId, HashSet<ProvinceId>> = HashMap::new();

        // 各ピクセルに対して集計する用のアキュムレータ
        struct Accum {
            count: u64,
            sum_x: f64,
            sum_y: f64,
            min_x: u32,
            min_y: u32,
            max_x: u32,
            max_y: u32,
        }

        let mut accumulators: HashMap<ProvinceId, Accum> = HashMap::new();

        // 全ピクセルを走査
        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                let r = pixels[idx];
                let g = pixels[idx + 1];
                let b = pixels[idx + 2];
                let key = (r as u32) << 16 | (g as u32) << 8 | b as u32;

                let Some(&id) = color_id_map.get(&key) else {
                    continue; // 未知の色はスキップ
                };

                // アキュムレータを更新
                let acc = accumulators.entry(id).or_insert(Accum {
                    count: 0,
                    sum_x: 0.0,
                    sum_y: 0.0,
                    min_x: x,
                    min_y: y,
                    max_x: x,
                    max_y: y,
                });
                acc.count += 1;
                acc.sum_x += x as f64;
                acc.sum_y += y as f64;
                acc.min_x = acc.min_x.min(x);
                acc.min_y = acc.min_y.min(y);
                acc.max_x = acc.max_x.max(x);
                acc.max_y = acc.max_y.max(y);

                // 4近傍の隣接チェック
                let neighbors = [
                    (x.wrapping_sub(1), y),
                    (x + 1, y),
                    (x, y.wrapping_sub(1)),
                    (x, y + 1),
                ];

                for (nx, ny) in neighbors {
                    if nx >= width || ny >= height {
                        continue;
                    }
                    let nidx = ((ny * width + nx) * 3) as usize;
                    let nr = pixels[nidx];
                    let ng = pixels[nidx + 1];
                    let nb = pixels[nidx + 2];
                    let nkey = (nr as u32) << 16 | (ng as u32) << 8 | nb as u32;

                    if nkey != key {
                        if let Some(&nid) = color_id_map.get(&nkey) {
                            adjacency.entry(id).or_default().insert(nid);
                        }
                    }
                }
            }
        }

        // アキュムレータからProvinceDataを生成
        let mut spatial_entries = Vec::new();
        for (&id, acc) in &accumulators {
            let centroid = [acc.sum_x / acc.count as f64, acc.sum_y / acc.count as f64];
            let color_key = color_id_map
                .iter()
                .find(|(_, &v)| v == id)
                .map(|(&k, _)| k)
                .unwrap_or(0);
            let color = ProvinceColor::from_key(color_key);

            let data = ProvinceData {
                id,
                color,
                pixel_count: acc.count as u32,
                sum_x: acc.sum_x as u64,
                sum_y: acc.sum_y as u64,
                centroid,
                bounds: [acc.min_x, acc.min_y, acc.max_x, acc.max_y],
            };
            provinces.insert(id, data);
            spatial_entries.push(ProvinceSpatialEntry { id, centroid });
        }

        let spatial_index = RTree::bulk_load(spatial_entries);

        Self {
            provinces,
            adjacency,
            color_to_id: color_id_map.clone(),
            spatial_index,
        }
    }

    /// RGB色からプロヴィンスIDを取得する。
    pub fn id_from_color(&self, color: &ProvinceColor) -> Option<ProvinceId> {
        self.color_to_id.get(&color.to_key()).copied()
    }

    /// プロヴィンスIDからデータを取得する。
    pub fn get_province(&self, id: ProvinceId) -> Option<&ProvinceData> {
        self.provinces.get(&id)
    }

    /// プロヴィンスIDの隣接IDリストを取得する。
    pub fn neighbors(&self, id: ProvinceId) -> Option<&HashSet<ProvinceId>> {
        self.adjacency.get(&id)
    }

    /// 座標の近傍にあるプロヴィンスID候補を返す (空間インデックス検索)。
    pub fn nearest_province(&self, x: f64, y: f64) -> Option<ProvinceId> {
        self.spatial_index
            .nearest_neighbor(&[x, y])
            .map(|entry| entry.id)
    }

    /// 全プロヴィンス数。
    pub fn province_count(&self) -> usize {
        self.provinces.len()
    }

    /// 全プロヴィンスのイテレータ。
    pub fn provinces(&self) -> impl Iterator<Item = (&ProvinceId, &ProvinceData)> {
        self.provinces.iter()
    }

    /// 特定のピクセルが変更された際に、統計情報 (面積、重心) を差分更新する。
    /// 隣接関係の更新は重いため、ストローク終了時などの一括更新を推奨する。
    pub fn update_pixel(
        &mut self,
        x: u32,
        y: u32,
        old_id: Option<ProvinceId>,
        new_id: Option<ProvinceId>,
    ) {
        if old_id == new_id {
            return;
        }

        if let Some(oid) = old_id {
            if let Some(data) = self.provinces.get_mut(&oid) {
                if data.pixel_count > 0 {
                    data.pixel_count -= 1;
                    data.sum_x = data.sum_x.saturating_sub(x as u64);
                    data.sum_y = data.sum_y.saturating_sub(y as u64);
                    if data.pixel_count > 0 {
                        data.centroid = [
                            data.sum_x as f64 / data.pixel_count as f64,
                            data.sum_y as f64 / data.pixel_count as f64,
                        ];
                    }
                }
            }
        }

        if let Some(nid) = new_id {
            if let Some(data) = self.provinces.get_mut(&nid) {
                data.pixel_count += 1;
                data.sum_x += x as u64;
                data.sum_y += y as u64;
                data.centroid = [
                    data.sum_x as f64 / data.pixel_count as f64,
                    data.sum_y as f64 / data.pixel_count as f64,
                ];
                data.bounds[0] = data.bounds[0].min(x);
                data.bounds[1] = data.bounds[1].min(y);
                data.bounds[2] = data.bounds[2].max(x);
                data.bounds[3] = data.bounds[3].max(y);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_province_color_key_roundtrip() {
        let color = ProvinceColor::new(128, 64, 255);
        let key = color.to_key();
        let back = ProvinceColor::from_key(key);
        assert_eq!(color, back);
    }

    #[test]
    fn test_build_simple_graph() {
        // 2x2 の画像: 左半分は赤(ID=1)、右半分は青(ID=2)
        let pixels: Vec<u8> = vec![255, 0, 0, 0, 0, 255, 255, 0, 0, 0, 0, 255];
        let mut color_id_map = HashMap::new();
        let red_key = (255u32) << 16;
        let blue_key = 255u32;
        color_id_map.insert(red_key, 1u32);
        color_id_map.insert(blue_key, 2u32);

        let graph = ProvinceGraph::build_from_pixels(&pixels, 2, 2, &color_id_map);

        assert_eq!(graph.province_count(), 2);
        assert!(graph.neighbors(1).unwrap().contains(&2));
        assert!(graph.neighbors(2).unwrap().contains(&1));
    }
}
