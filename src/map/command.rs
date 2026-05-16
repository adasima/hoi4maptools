use crate::core::command::Command;
use crate::map::ProjectState;
use crate::map::graph::{ProvinceColor, ProvinceId};
use anyhow::Result;
use eframe::egui;

/// 塗りつぶしコマンド。
/// 指定された色 (from_color) の全ピクセルを別の色 (to_color) に置き換える。
#[derive(Debug)]
pub struct FillCommand {
    pub from_color: ProvinceColor,
    pub to_color: ProvinceColor,
}

impl Command<ProjectState> for FillCommand {
    fn execute(&mut self, project: &mut ProjectState) -> Result<()> {
        Self::apply(project, self.from_color, self.to_color)
    }

    fn undo(&mut self, project: &mut ProjectState) -> Result<()> {
        Self::apply(project, self.to_color, self.from_color)
    }

    fn description(&self) -> &str {
        "プロヴィンスの塗りつぶし"
    }
}

impl FillCommand {
    fn apply(project: &mut ProjectState, from: ProvinceColor, to: ProvinceColor) -> Result<()> {
        if from == to { return Ok(()); }
        
        let width = project.width;
        let height = project.height;
        let old_id = project.graph.id_from_color(&from);
        let new_id = project.graph.id_from_color(&to);

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                if project.pixels[idx] == from.r && project.pixels[idx + 1] == from.g && project.pixels[idx + 2] == from.b {
                    project.pixels[idx] = to.r;
                    project.pixels[idx + 1] = to.g;
                    project.pixels[idx + 2] = to.b;
                    // リアルタイムグラフ更新
                    project.graph.update_pixel(x, y, old_id, new_id);
                }
            }
        }

        // 全体を dirty に設定
        project.dirty_rect = Some(egui::Rect::from_min_max(
            egui::pos2(0.0, 0.0),
            egui::pos2(width as f32, height as f32)
        ));
        
        Ok(())
    }
}

/// ブラシストロークコマンド。
/// ドラッグ中の継続的な描画を1つの Undo 単位としてまとめるためのコマンド。
#[derive(Debug)]
pub struct PaintStrokeCommand {
    pub color: ProvinceColor,
    /// 変更したピクセルの元の状態 (x, y, old_color)
    pub history: Vec<(u32, u32, ProvinceColor)>,
}

impl Command<ProjectState> for PaintStrokeCommand {
    fn execute(&mut self, project: &mut ProjectState) -> Result<()> {
        let new_id = project.graph.id_from_color(&self.color);
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for (x, y, old_color) in &self.history {
            let idx = ((y * project.width + x) * 3) as usize;
            project.pixels[idx] = self.color.r;
            project.pixels[idx + 1] = self.color.g;
            project.pixels[idx + 2] = self.color.b;
            
            let old_id = project.graph.id_from_color(old_color);
            project.graph.update_pixel(*x, *y, old_id, new_id);

            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }

        if !self.history.is_empty() {
            project.dirty_rect = Some(egui::Rect::from_min_max(
                egui::pos2(min_x as f32, min_y as f32),
                egui::pos2(max_x as f32 + 1.0, max_y as f32 + 1.0)
            ));
        }

        Ok(())
    }

    fn undo(&mut self, project: &mut ProjectState) -> Result<()> {
        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;
        
        let new_id = project.graph.id_from_color(&self.color);

        for (x, y, old_color) in &self.history {
            let idx = ((y * project.width + x) * 3) as usize;
            project.pixels[idx] = old_color.r;
            project.pixels[idx + 1] = old_color.g;
            project.pixels[idx + 2] = old_color.b;
            
            let old_id = project.graph.id_from_color(old_color);
            project.graph.update_pixel(*x, *y, new_id, old_id);

            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }

        if !self.history.is_empty() {
            project.dirty_rect = Some(egui::Rect::from_min_max(
                egui::pos2(min_x as f32, min_y as f32),
                egui::pos2(max_x as f32 + 1.0, max_y as f32 + 1.0)
            ));
        }

        Ok(())
    }

    fn description(&self) -> &str {
        "ブラシ描画"
    }
}

/// 新規プロヴィンス作成コマンド。
/// 指定された地点の連結成分を新しい ID/色 で置き換える。
#[derive(Debug)]
pub struct NewProvinceCommand {
    pub new_color: ProvinceColor,
    pub old_color: ProvinceColor,
    pub province_type: crate::map::definition::ProvinceType,
    pub terrain: String,
    pub continent: u32,
    /// 変更されたピクセルのリスト
    pub history: Vec<(u32, u32)>,
}

impl Command<ProjectState> for NewProvinceCommand {
    fn execute(&mut self, project: &mut ProjectState) -> Result<()> {
        let new_id = project.definitions.add_province(
            self.new_color,
            self.province_type.clone(),
            &self.terrain,
            self.continent,
        );
        let old_id = project.graph.id_from_color(&self.old_color);

        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for (x, y) in &self.history {
            let idx = ((*y * project.width + *x) * 3) as usize;
            project.pixels[idx] = self.new_color.r;
            project.pixels[idx + 1] = self.new_color.g;
            project.pixels[idx + 2] = self.new_color.b;

            project.graph.update_pixel(*x, *y, old_id, Some(new_id));

            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }

        if !self.history.is_empty() {
            project.dirty_rect = Some(egui::Rect::from_min_max(
                egui::pos2(min_x as f32, min_y as f32),
                egui::pos2(max_x as f32 + 1.0, max_y as f32 + 1.0)
            ));
        }

        Ok(())
    }

    fn undo(&mut self, project: &mut ProjectState) -> Result<()> {
        let new_id = project.graph.id_from_color(&self.new_color);
        let old_id = project.graph.id_from_color(&self.old_color);

        let mut min_x = u32::MAX;
        let mut min_y = u32::MAX;
        let mut max_x = 0u32;
        let mut max_y = 0u32;

        for (x, y) in &self.history {
            let idx = ((*y * project.width + *x) * 3) as usize;
            project.pixels[idx] = self.old_color.r;
            project.pixels[idx + 1] = self.old_color.g;
            project.pixels[idx + 2] = self.old_color.b;

            project.graph.update_pixel(*x, *y, new_id, old_id);

            min_x = min_x.min(*x);
            min_y = min_y.min(*y);
            max_x = max_x.max(*x);
            max_y = max_y.max(*y);
        }

        // Definition から新 ID を削除 (厳密には definition.csv の整合性は維持)
        if let Some(id) = new_id {
            project.definitions.remove(id);
        }

        if !self.history.is_empty() {
            project.dirty_rect = Some(egui::Rect::from_min_max(
                egui::pos2(min_x as f32, min_y as f32),
                egui::pos2(max_x as f32 + 1.0, max_y as f32 + 1.0)
            ));
        }

        Ok(())
    }

    fn description(&self) -> &str {
        "新規プロヴィンス作成"
    }
}
