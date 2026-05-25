/// メインアプリケーション。
/// WorldSmithApp は eframe::App を実装し、UIループを駆動する。
use eframe::egui;
use std::path::PathBuf;

use crate::core::command::CommandStack;
use crate::core::fs::ParadoxPathResolver;
use crate::core::i18n::I18n;
use crate::core::rng::WorldRng;
use crate::map::definition::DefinitionTable;
use crate::map::graph::{ProvinceColor, ProvinceGraph, ProvinceId};
use crate::map::ProjectState;
use crate::painter::{
    hex_cell_to_polygon, map_pos_to_hex_cell, GridMode, HexGridConfig, PainterPoint,
};
use crate::renderer::MapViewport;

/// ホバー/クリックで表示するプロヴィンスの情報
struct InspectorInfo {
    id: ProvinceId,
    color: ProvinceColor,
    province_type: String,
    terrain: String,
    pixel_count: u32,
    neighbor_count: usize,
}

/// ツール種別。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActiveTool {
    Inspect,
    Eyedropper,
    Fill,
    Brush,
    NewProvince,
    BoxSelect,
}

/// メインアプリケーション構造体。
pub struct WorldSmithApp {
    /// コマンド履歴 (Undo/Redo)
    command_stack: CommandStack<ProjectState>,
    /// ファイルシステムリゾルバ
    path_resolver: Option<ParadoxPathResolver>,
    /// ビューポート(パン/ズーム管理)
    viewport: MapViewport,
    /// テクスチャハンドル (egui)
    map_texture: Option<egui::TextureHandle>,
    /// 読み込まれたプロジェクト
    project: Option<ProjectState>,
    /// インスペクター情報
    inspector: Option<InspectorInfo>,
    /// ステータスバーのメッセージ
    status_message: String,
    /// 決定論的乱数生成器
    rng: WorldRng,
    /// シード入力用バッファ
    rng_seed_input: String,
    /// 国際化 (i18n)
    i18n: I18n,
    /// 現在選択中のツール
    active_tool: ActiveTool,
    /// ブラシサイズ (5段階: 1, 2, 4, 8, 16 など)
    brush_size: u32,
    /// 現在のドラッグ中のストローク
    current_stroke: Option<crate::map::command::PaintStrokeCommand>,
    /// 明示的に選択されたプロヴィンス群
    selected_provinces: std::collections::HashSet<ProvinceId>,
    /// ドラッグ選択の矩形
    #[allow(dead_code)]
    selection_rect: Option<egui::Rect>,
    /// 現在描画・塗りつぶしに使う色
    current_brush_color: Option<ProvinceColor>,
    /// Painter: ボロノイポイント
    painter_points: Vec<PainterPoint>,
    /// Painter: グリッド/ポイントモード
    grid_mode: GridMode,
    /// Painter: Hex グリッド設定
    hex_config: HexGridConfig,
}

impl WorldSmithApp {
    pub fn new(cc: &eframe::CreationContext<'_>) -> Self {
        setup_custom_fonts(&cc.egui_ctx);
        let i18n_assets = std::env::current_dir()
            .unwrap_or_default()
            .join("assets")
            .join("i18n");
        let i18n = I18n::from_assets_dir(i18n_assets, "ja-JP").unwrap_or_else(|_| {
            log::warn!("i18n 初期化に失敗しました。キー文字列をそのまま表示します。");
            I18n::empty("ja-JP")
        });
        let default_seed = 42u64;
        Self {
            command_stack: CommandStack::new(500),
            path_resolver: None,
            viewport: MapViewport::new(),
            map_texture: None,
            project: None,
            inspector: None,
            status_message: "World Smith へようこそ。ファイル → マップを開く で開始してください。"
                .to_string(),
            rng: WorldRng::new(default_seed),
            rng_seed_input: default_seed.to_string(),
            i18n,
            active_tool: ActiveTool::Inspect,
            brush_size: 1,
            current_stroke: None,
            selected_provinces: std::collections::HashSet::new(),
            selection_rect: None,
            current_brush_color: None,
            painter_points: Vec::new(),
            grid_mode: GridMode::None,
            hex_config: HexGridConfig::default(),
        }
    }
}

/// 日本語フォントをセットアップする。
fn setup_custom_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();

    // Windows システムフォントのパス。msgothic.ttc を使用。
    let font_path = "C:\\Windows\\Fonts\\msgothic.ttc";
    if let Ok(font_data) = std::fs::read(font_path) {
        fonts.font_data.insert(
            "japanese_font".to_owned(),
            std::sync::Arc::new(egui::FontData::from_owned(font_data)),
        );

        // プロポーショナルフォントとモノスペースフォントの両方に日本語フォントを優先的に割り当て
        fonts
            .families
            .entry(egui::FontFamily::Proportional)
            .or_default()
            .insert(0, "japanese_font".to_owned());

        fonts
            .families
            .entry(egui::FontFamily::Monospace)
            .or_default()
            .insert(0, "japanese_font".to_owned());
    } else {
        log::warn!("日本語フォントが見つかりませんでした: {}", font_path);
    }

    ctx.set_fonts(fonts);
}

impl WorldSmithApp {
    fn load_map_folder(&mut self, folder: PathBuf, ctx: &egui::Context) {
        self.status_message = format!("読み込み中: {}...", folder.display());

        // VFS セットアップ (現時点ではバニラのみ)
        self.path_resolver = Some(ParadoxPathResolver::vanilla_only(folder.clone()));

        // definition.csv を読み込む
        let def_path = folder.join("definition.csv");
        let definitions = match DefinitionTable::load_from_file(&def_path) {
            Ok(d) => d,
            Err(e) => {
                self.status_message = format!("❌ definition.csv の読み込みに失敗: {}", e);
                log::error!("{}", self.status_message);
                return;
            }
        };

        // provinces.bmp を読み込む
        let bmp_path = folder.join("provinces.bmp");
        let (width, height, pixels) = match crate::map_loader::load_provinces_bmp(&bmp_path) {
            Ok(v) => v,
            Err(e) => {
                self.status_message = format!("❌ provinces.bmp の読み込みに失敗: {}", e);
                log::error!("{}", self.status_message);
                return;
            }
        };

        // プロヴィンスグラフを構築
        let color_id_map = definitions.color_id_map().clone();
        let graph = ProvinceGraph::build_from_pixels(&pixels, width, height, &color_id_map);

        log::info!("グラフ構築完了: {} プロヴィンス", graph.province_count());

        // テクスチャを作成
        let color_image = crate::map_loader::pixels_to_color_image(&pixels, width, height);
        let texture = ctx.load_texture(
            "provinces_map",
            color_image,
            egui::TextureOptions::NEAREST, // 最近傍補間 (色の正確性のため)
        );
        self.map_texture = Some(texture);

        // ビューポートをリセット
        self.viewport = MapViewport::new();
        self.viewport.dirty_tracker.mark_all_dirty(width, height);

        // プロジェクト状態を保存
        self.project = Some(ProjectState {
            pixels,
            width,
            height,
            definitions,
            graph,
            project_dir: folder,
            dirty_rect: None,
        });

        if let Some(project) = &self.project {
            self.status_message = format!(
                "✅ マップ読み込み完了: {}x{} | {} プロヴィンス",
                width,
                height,
                project.graph.province_count()
            );
        }

        // コマンド履歴をクリア
        self.command_stack.clear();
    }

    /// デバッグエクスポート: 現在のピクセルデータを BMP に書き出す。
    fn debug_export(&self) {
        let Some(project) = &self.project else {
            log::warn!("プロジェクトが読み込まれていません");
            return;
        };

        if let Some(path) = rfd::FileDialog::new()
            .set_title("デバッグエクスポート - provinces.bmp")
            .add_filter("BMP", &["bmp"])
            .set_file_name("provinces_debug.bmp")
            .save_file()
        {
            match crate::map_loader::save_provinces_bmp(
                &path,
                &project.pixels,
                project.width,
                project.height,
            ) {
                Ok(_) => log::info!("デバッグエクスポート完了: {}", path.display()),
                Err(e) => log::error!("デバッグエクスポートに失敗: {}", e),
            }
        }
    }

    /// 現在のプロジェクトを definition.csv / provinces.bmp に書き出す。
    fn save_project(&mut self) {
        let Some(project) = &self.project else {
            self.status_message = "プロジェクトが読み込まれていません。".to_string();
            log::warn!("{}", self.status_message);
            return;
        };

        let def_path = project.project_dir.join("definition.csv");
        if let Err(e) = project.definitions.write_to_file(&def_path) {
            self.status_message = format!("❌ definition.csv の書き出しに失敗: {}", e);
            log::error!("{}", self.status_message);
            return;
        }

        let bmp_path = project.project_dir.join("provinces.bmp");
        if let Err(e) = crate::map_loader::save_provinces_bmp(
            &bmp_path,
            &project.pixels,
            project.width,
            project.height,
        ) {
            self.status_message = format!("❌ provinces.bmp の書き出しに失敗: {}", e);
            log::error!("{}", self.status_message);
            return;
        }

        self.status_message = "✅ プロジェクトを保存しました。".to_string();
        log::info!("{}", self.status_message);
    }

    /// マップ座標からプロヴィンスIDと色を取得する。
    fn pick_province_at(&self, map_pos: egui::Pos2) -> Option<(ProvinceId, ProvinceColor)> {
        let Some(project) = &self.project else {
            return None;
        };
        let x = map_pos.x as i32;
        let y = map_pos.y as i32;

        if x < 0 || y < 0 || x >= project.width as i32 || y >= project.height as i32 {
            return None;
        }

        let x = x as u32;
        let y = y as u32;
        let idx = ((y * project.width + x) * 3) as usize;
        let r = project.pixels[idx];
        let g = project.pixels[idx + 1];
        let b = project.pixels[idx + 2];
        let color = ProvinceColor::new(r, g, b);

        let project = &self.project.as_ref()?;
        let id = project.graph.id_from_color(&color)?;
        Some((id, color))
    }

    /// マウス位置からプロヴィンス情報を更新する。
    fn update_inspector(&mut self, map_pos: egui::Pos2) {
        let Some(project) = &self.project else {
            self.inspector = None;
            return;
        };

        if let Some((id, color)) = self.pick_province_at(map_pos) {
            let (province_type, terrain) = project
                .definitions
                .get(id)
                .map(|d| (d.province_type.as_str().to_string(), d.terrain.clone()))
                .unwrap_or_else(|| ("不明".to_string(), "不明".to_string()));
            let pixel_count = project
                .graph
                .get_province(id)
                .map(|p| p.pixel_count)
                .unwrap_or(0);
            let neighbor_count = project.graph.neighbors(id).map(|n| n.len()).unwrap_or(0);

            self.inspector = Some(InspectorInfo {
                id,
                color,
                province_type,
                terrain,
                pixel_count,
                neighbor_count,
            });
        } else {
            self.inspector = None;
        }
    }

    /// プロヴィンス全体を別の色で塗りつぶす。
    #[allow(dead_code)]
    fn fill_province_color(
        &mut self,
        from_color: ProvinceColor,
        to_color: ProvinceColor,
        ctx: &egui::Context,
    ) {
        if from_color == to_color {
            return;
        }

        let Some(project) = &mut self.project else {
            return;
        };

        let mut changed = false;
        let width = project.width;
        let height = project.height;

        for y in 0..height {
            for x in 0..width {
                let idx = ((y * width + x) * 3) as usize;
                let r = project.pixels[idx];
                let g = project.pixels[idx + 1];
                let b = project.pixels[idx + 2];

                if r == from_color.r && g == from_color.g && b == from_color.b {
                    project.pixels[idx] = to_color.r;
                    project.pixels[idx + 1] = to_color.g;
                    project.pixels[idx + 2] = to_color.b;
                    changed = true;
                }
            }
        }

        if !changed {
            return;
        }

        // プロヴィンスグラフを再構築して整合性を保つ。
        let color_id_map = project.definitions.color_id_map().clone();
        project.graph =
            ProvinceGraph::build_from_pixels(&project.pixels, width, height, &color_id_map);

        // テクスチャを更新。
        let color_image = crate::map_loader::pixels_to_color_image(&project.pixels, width, height);
        let texture = ctx.load_texture("provinces_map", color_image, egui::TextureOptions::NEAREST);
        self.map_texture = Some(texture);
        self.viewport.dirty_tracker.mark_all_dirty(width, height);
    }

    /// Voronoiポイントモードが有効な場合、クリック位置に PainterPoint を追加する。
    fn handle_painter_point_click(&mut self, map_pos: egui::Pos2) {
        if self.grid_mode != GridMode::VoronoiPoints {
            return;
        }
        self.painter_points.push(PainterPoint {
            x: map_pos.x,
            y: map_pos.y,
        });
    }

    /// クリック位置から連結成分単位で新規プロヴィンスを切り出す。
    fn create_new_province_at(&mut self, map_pos: egui::Pos2, _ctx: &egui::Context) {
        let Some(project) = &mut self.project else {
            self.status_message = "マップが読み込まれていません。".to_string();
            return;
        };

        let x = map_pos.x as i32;
        let y = map_pos.y as i32;
        if x < 0 || y < 0 || x >= project.width as i32 || y >= project.height as i32 {
            return;
        }
        let x = x as u32;
        let y = y as u32;

        // クリックした元の色を取得
        let idx = ((y * project.width + x) * 3) as usize;
        let src_color = ProvinceColor::new(
            project.pixels[idx],
            project.pixels[idx + 1],
            project.pixels[idx + 2],
        );

        // 既存定義からタイプや地形を継承 (無ければデフォルト)
        let existing_id = project.graph.id_from_color(&src_color);
        let (province_type, terrain, continent) = if let Some(id) = existing_id {
            if let Some(def) = project.definitions.get(id) {
                (
                    def.province_type.clone(),
                    def.terrain.clone(),
                    def.continent,
                )
            } else {
                (
                    crate::map::definition::ProvinceType::Land,
                    "unknown".to_string(),
                    0,
                )
            }
        } else {
            (
                crate::map::definition::ProvinceType::Land,
                "unknown".to_string(),
                0,
            )
        };

        // 未使用の RGB を割り当てて新規プロヴィンス定義を追加
        let Some(new_color) = project.definitions.allocate_unused_color() else {
            self.status_message = "新しいRGBカラーを確保できませんでした。".to_string();
            return;
        };
        // allocate_unused_color で确保した色を使用する。実際の ID 割り当てはコマンド内で行う。

        // BFS による4近傍フラッドフィルで、この連結成分を取得する
        let width = project.width;
        let height = project.height;
        let mut queue = std::collections::VecDeque::new();
        let mut visited = vec![false; (width * height) as usize];
        let mut affected_pixels = Vec::new();

        let start_index = (y * width + x) as usize;
        queue.push_back((x, y));
        visited[start_index] = true;

        while let Some((cx, cy)) = queue.pop_front() {
            let idx = ((cy * width + cx) * 3) as usize;
            let r = project.pixels[idx];
            let g = project.pixels[idx + 1];
            let b = project.pixels[idx + 2];

            if r == src_color.r && g == src_color.g && b == src_color.b {
                affected_pixels.push((cx, cy));

                let neighbors = [
                    (cx.wrapping_sub(1), cy),
                    (cx + 1, cy),
                    (cx, cy.wrapping_sub(1)),
                    (cx, cy + 1),
                ];
                for (nx, ny) in neighbors {
                    if nx >= width || ny >= height {
                        continue;
                    }
                    let n_index = (ny * width + nx) as usize;
                    if !visited[n_index] {
                        visited[n_index] = true;
                        queue.push_back((nx, ny));
                    }
                }
            }
        }

        // コマンド実行
        let cmd = Box::new(crate::map::command::NewProvinceCommand {
            new_color,
            old_color: src_color,
            province_type,
            terrain,
            continent,
            history: affected_pixels,
        });

        let _ = self.command_stack.execute(cmd, project);

        self.status_message = format!(
            "🆕 新規プロヴィンスを作成: 色 ({}, {}, {})",
            new_color.r, new_color.g, new_color.b
        );
    }

    /// ブラシストロークの終了。コマンドを確定させて履歴に積む。
    fn finalize_stroke(&mut self) {
        if let Some(stroke) = self.current_stroke.take() {
            if !stroke.history.is_empty() {
                self.command_stack.push_to_undo(Box::new(stroke));
            }
        }
    }

    /// ツール実行用のメイン処理。
    fn handle_tool_action(
        &mut self,
        map_pos: egui::Pos2,
        ctx: &egui::Context,
        is_new_stroke: bool,
    ) {
        let Some(project) = &mut self.project else {
            return;
        };

        match self.active_tool {
            ActiveTool::Brush => {
                if is_new_stroke {
                    self.current_stroke = Some(crate::map::command::PaintStrokeCommand {
                        color: self
                            .current_brush_color
                            .unwrap_or(ProvinceColor::new(255, 255, 255)),
                        history: Vec::new(),
                    });
                }

                if let (Some(stroke), Some(brush_color)) =
                    (&mut self.current_stroke, &self.current_brush_color)
                {
                    let radius = self.brush_size as f32;
                    let r2 = radius * radius;
                    let width = project.width;
                    let height = project.height;

                    let min_x = (map_pos.x - radius).max(0.0) as u32;
                    let min_y = (map_pos.y - radius).max(0.0) as u32;
                    let max_x = (map_pos.x + radius).min(width as f32 - 1.0) as u32;
                    let max_y = (map_pos.y + radius).min(height as f32 - 1.0) as u32;

                    let mut changed = false;
                    let new_id = project.graph.id_from_color(brush_color);

                    for py in min_y..=max_y {
                        for px in min_x..=max_x {
                            let dx = px as f32 - map_pos.x;
                            let dy = py as f32 - map_pos.y;
                            if dx * dx + dy * dy <= r2 {
                                let idx = ((py * width + px) * 3) as usize;
                                let old_color = ProvinceColor::new(
                                    project.pixels[idx],
                                    project.pixels[idx + 1],
                                    project.pixels[idx + 2],
                                );

                                if old_color != *brush_color {
                                    stroke.history.push((px, py, old_color));
                                    project.pixels[idx] = brush_color.r;
                                    project.pixels[idx + 1] = brush_color.g;
                                    project.pixels[idx + 2] = brush_color.b;

                                    let old_id = project.graph.id_from_color(&old_color);
                                    project.graph.update_pixel(px, py, old_id, new_id);
                                    changed = true;
                                }
                            }
                        }
                    }

                    if changed {
                        project.dirty_rect = Some(egui::Rect::from_min_max(
                            egui::pos2(min_x as f32, min_y as f32),
                            egui::pos2(max_x as f32 + 1.0, max_y as f32 + 1.0),
                        ));
                    }
                }
            }
            _ => {
                if is_new_stroke {
                    self.handle_tool_click(map_pos, ctx);
                }
            }
        }
    }

    /// ツール実行用のクリック処理。
    fn handle_tool_click(&mut self, map_pos: egui::Pos2, ctx: &egui::Context) {
        let Some((id, color)) = self.pick_province_at(map_pos) else {
            return;
        };

        match self.active_tool {
            ActiveTool::Inspect => {
                self.current_brush_color = Some(color);
                self.status_message = format!(
                    "選択中プロヴィンス: ID {} ({}, {}, {})",
                    id, color.r, color.g, color.b
                );
            }
            ActiveTool::Eyedropper => {
                self.current_brush_color = Some(color);
                self.status_message = format!(
                    "スポイト: ID {} ({}, {}, {}) を選択しました",
                    id, color.r, color.g, color.b
                );
            }
            ActiveTool::Fill => {
                if let Some(brush_color) = &self.current_brush_color {
                    let cmd = Box::new(crate::map::command::FillCommand {
                        from_color: color,
                        to_color: *brush_color,
                    });
                    if let Some(project) = &mut self.project {
                        let _ = self.command_stack.execute(cmd, project);
                    }
                } else {
                    self.status_message =
                        "塗りつぶしには先にスポイトで色を選択してください。".to_string();
                }
            }
            ActiveTool::NewProvince => {
                self.create_new_province_at(map_pos, ctx);
            }
            ActiveTool::Brush => {} // Brush は別ロジック
            ActiveTool::BoxSelect => {}
        }

        // Painter のポイントモード処理（ツールに関係なく動作）
        self.handle_painter_point_click(map_pos);
    }

    /// メニューバーの描画。
    fn draw_menu_bar(&mut self, ui: &mut egui::Ui, ctx: &egui::Context) {
        egui::menu::bar(ui, |ui| {
            ui.menu_button(self.i18n.tr("menu.file"), |ui| {
                if ui
                    .button(self.i18n.tr("menu.file.open_map_folder"))
                    .clicked()
                {
                    if let Some(folder) = rfd::FileDialog::new()
                        .set_title("HoI4 マップフォルダを選択")
                        .pick_folder()
                    {
                        self.load_map_folder(folder, ctx);
                    }
                    ui.close_menu();
                }
                ui.separator();
                if ui.button(self.i18n.tr("menu.file.debug_export")).clicked() {
                    self.debug_export();
                    ui.close_menu();
                }
                if ui.button("プロジェクトを上書き保存").clicked() {
                    self.save_project();
                    ui.close_menu();
                }
            });
            ui.menu_button(self.i18n.tr("menu.edit"), |ui| {
                let can_undo = self.command_stack.can_undo();
                let can_redo = self.command_stack.can_redo();
                if ui
                    .add_enabled(can_undo, egui::Button::new(self.i18n.tr("menu.edit.undo")))
                    .clicked()
                {
                    if let Some(project) = &mut self.project {
                        let _ = self.command_stack.undo(project);
                    }
                    ui.close_menu();
                }
                if ui
                    .add_enabled(can_redo, egui::Button::new(self.i18n.tr("menu.edit.redo")))
                    .clicked()
                {
                    if let Some(project) = &mut self.project {
                        let _ = self.command_stack.redo(project);
                    }
                    ui.close_menu();
                }
            });
            ui.menu_button(self.i18n.tr("menu.settings"), |ui| {
                ui.label(self.i18n.tr("settings.rng_seed_label"));
                let response = ui.text_edit_singleline(&mut self.rng_seed_input);
                if response.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) {
                    if let Ok(seed) = self.rng_seed_input.parse::<u64>() {
                        self.rng.set_seed(seed);
                        self.status_message =
                            format!("{} {}", self.i18n.tr("settings.rng_seed_set"), seed);
                    } else {
                        self.status_message = self.i18n.tr("settings.rng_seed_invalid");
                    }
                }
                if ui
                    .button(self.i18n.tr("settings.rng_seed_generate_random"))
                    .clicked()
                {
                    let new_seed = self.rng.next_u64();
                    self.rng.set_seed(new_seed);
                    self.rng_seed_input = new_seed.to_string();
                    self.status_message = format!(
                        "{} {}",
                        self.i18n.tr("settings.rng_seed_generated"),
                        new_seed
                    );
                }
            });
        });
    }

    /// 左パネル: ツール群。
    fn draw_left_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🛠 ツール");
        ui.separator();

        ui.label("🎨 The Painter");
        ui.horizontal(|ui| {
            ui.selectable_value(&mut self.active_tool, ActiveTool::Inspect, "👁 Inspect");
            ui.selectable_value(
                &mut self.active_tool,
                ActiveTool::Eyedropper,
                "🎯 Eyedropper",
            );
            ui.selectable_value(&mut self.active_tool, ActiveTool::Brush, "🖌 Brush");
            ui.selectable_value(&mut self.active_tool, ActiveTool::Fill, "🪣 Fill");
            ui.selectable_value(&mut self.active_tool, ActiveTool::BoxSelect, "🔲 Select");
        });

        if self.active_tool == ActiveTool::Brush {
            ui.horizontal(|ui| {
                ui.label("サイズ:");
                for &size in &[1, 2, 4, 8, 16] {
                    ui.selectable_value(&mut self.brush_size, size, size.to_string());
                }
            });
        }

        ui.horizontal(|ui| {
            ui.selectable_value(
                &mut self.active_tool,
                ActiveTool::NewProvince,
                "➕ New Province",
            );
        });

        ui.separator();
        ui.label("📐 Painter モード");
        ui.checkbox(
            &mut (self.grid_mode == GridMode::VoronoiPoints),
            "Voronoiポイントモード",
        )
        .changed()
        .then(|| {
            self.grid_mode = if self.grid_mode == GridMode::VoronoiPoints {
                GridMode::None
            } else {
                GridMode::VoronoiPoints
            };
        });

        ui.checkbox(
            &mut (self.grid_mode == GridMode::HexGrid),
            "Hexグリッド表示",
        )
        .changed()
        .then(|| {
            self.grid_mode = if self.grid_mode == GridMode::HexGrid {
                GridMode::None
            } else {
                GridMode::HexGrid
            };
        });

        ui.add(
            egui::Slider::new(&mut self.hex_config.cell_size, 16.0..=256.0).text("Hexセルサイズ"),
        );
        ui.add_enabled(false, egui::Button::new("ボロノイ・ブラシ"));
        if ui.button("マップ全体をHexで再生成").clicked() {
            let cmd = Box::new(crate::map::command::GenerateHexMapCommand::new(
                self.hex_config.clone(),
            ));
            if let Some(project) = &mut self.project {
                let _ = self.command_stack.execute(cmd, project);
            }
        }
        ui.add_enabled(false, egui::Button::new("シンメトリーモード"));
        ui.separator();

        ui.label("🔧 The Sculptor");
        ui.add_enabled(false, egui::Button::new("頂点編集"));
        ui.add_enabled(false, egui::Button::new("結合/分割"));
        ui.separator();

        ui.label("📐 The Logic");
        ui.add_enabled(false, egui::Button::new("ステート・グルーピング"));
        ui.add_enabled(false, egui::Button::new("エラー検知"));
        ui.separator();

        ui.label("⚖ The Balancer");
        ui.add_enabled(false, egui::Button::new("補給シミュレーション"));
        ui.add_enabled(false, egui::Button::new("資源バランス"));
    }

    /// 右パネル: プロヴィンスインスペクター。
    fn draw_right_panel(&mut self, ui: &mut egui::Ui) {
        ui.heading("🔍 インスペクター");
        ui.separator();

        if let Some(info) = &self.inspector {
            egui::Grid::new("inspector_grid")
                .num_columns(2)
                .spacing([10.0, 4.0])
                .show(ui, |ui| {
                    ui.label("プロヴィンスID:");
                    ui.strong(format!("{}", info.id));
                    ui.end_row();

                    ui.label("色 (RGB):");
                    let color_str =
                        format!("({}, {}, {})", info.color.r, info.color.g, info.color.b);
                    ui.horizontal(|ui| {
                        let color32 =
                            egui::Color32::from_rgb(info.color.r, info.color.g, info.color.b);
                        ui.colored_label(color32, "■");
                        ui.label(color_str);
                    });
                    ui.end_row();

                    ui.label("種別:");
                    ui.label(&info.province_type);
                    ui.end_row();

                    ui.label("地形:");
                    ui.label(&info.terrain);
                    ui.end_row();

                    ui.label("面積 (px):");
                    ui.label(format!("{}", info.pixel_count));
                    ui.end_row();

                    ui.label("隣接数:");
                    ui.label(format!("{}", info.neighbor_count));
                    ui.end_row();
                });
        } else {
            ui.label("マップ上をホバーすると情報が表示されます");
        }

        ui.separator();
        ui.heading("📑 レイヤー");
        // 後のフェーズでレイヤー管理UIを追加
        ui.label("(今後実装)");
        ui.separator();
        ui.heading("📦 一括編集");
        let selected_count = self.selected_provinces.len();
        if selected_count > 0 {
            ui.label(format!("{} 個のプロヴィンスを選択中", selected_count));

            // 簡易的な一括編集用ステート。
            // 実際は app.rs に持たせるのがベストですが、UIデモとして
            // 一時的な変更ならボタンクリック時のハードコードでも動作確認可能です。
            // 今回は一括で地形を 'plains' に変えるボタン等を用意します。

            ui.horizontal(|ui| {
                if ui.button("地形を 'plains' に統一").clicked() {
                    let cmd = Box::new(crate::map::command::EditProvincesCommand {
                        province_ids: self.selected_provinces.clone(),
                        new_terrain: Some("plains".to_string()),
                        new_province_type: None,
                        new_continent: None,
                        history: std::collections::HashMap::new(),
                    });
                    if let Some(project) = &mut self.project {
                        let _ = self.command_stack.execute(cmd, project);
                    }
                }

                if ui.button("地形を 'mountain' に統一").clicked() {
                    let cmd = Box::new(crate::map::command::EditProvincesCommand {
                        province_ids: self.selected_provinces.clone(),
                        new_terrain: Some("mountain".to_string()),
                        new_province_type: None,
                        new_continent: None,
                        history: std::collections::HashMap::new(),
                    });
                    if let Some(project) = &mut self.project {
                        let _ = self.command_stack.execute(cmd, project);
                    }
                }
            });
        } else {
            ui.label("プロヴィンスが選択されていません。\n(Select ツールでドラッグ)");
        }
    }

    /// 中央パネル: マップビューポート。
    fn draw_map_viewport(&mut self, ui: &mut egui::Ui) {
        let available_rect = ui.available_rect_before_wrap();

        // 入力処理: パン＆ズーム
        let response = ui.allocate_rect(available_rect, egui::Sense::click_and_drag());

        if response.dragged_by(egui::PointerButton::Middle)
            || (response.dragged_by(egui::PointerButton::Primary)
                && ui.input(|i| i.modifiers.shift))
        {
            let delta = response.drag_delta();
            self.viewport.offset += delta / self.viewport.zoom;
        }

        // ズーム (マウスホイール)
        let scroll_delta = ui.input(|i| i.smooth_scroll_delta.y);
        if scroll_delta != 0.0 {
            if scroll_delta > 0.0 {
                self.viewport.zoom_in();
            } else {
                self.viewport.zoom_out();
            }
        }

        let zoom = self.viewport.zoom;
        let offset = self.viewport.offset;

        // マップ描画
        if let Some(texture) = &mut self.map_texture {
            let tex_size = texture.size_vec2();
            let scaled_size = tex_size * zoom;

            let center = available_rect.center();
            let scaled_offset = offset * zoom;
            let top_left = egui::Pos2::new(
                center.x + scaled_offset.x - scaled_size.x / 2.0,
                center.y + scaled_offset.y - scaled_size.y / 2.0,
            );

            let image_rect = egui::Rect::from_min_size(top_left, scaled_size);

            // テクスチャを描画
            let painter = ui.painter_at(available_rect);
            painter.image(
                texture.id(),
                image_rect,
                egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
                egui::Color32::WHITE,
            );

            // テクスチャの更新 (もしあれば)
            if let Some(project) = &mut self.project {
                if project.dirty_rect.is_some() {
                    let color_image = crate::map_loader::pixels_to_color_image(
                        &project.pixels,
                        project.width,
                        project.height,
                    );
                    texture.set(color_image, egui::TextureOptions::NEAREST);
                    project.dirty_rect = None;
                }
            }

            // マウスホバー時のインスペクター更新
            if let Some(hover_pos) = response.hover_pos() {
                let map_pos = egui::Pos2::new(
                    (hover_pos.x - top_left.x) / zoom,
                    (hover_pos.y - top_left.y) / zoom,
                );
                self.update_inspector(map_pos);
            }

            // クリック/ドラッグ時のツール実行
            if response.dragged_by(egui::PointerButton::Primary) || response.clicked() {
                if let Some(click_pos) = response.interact_pointer_pos() {
                    let map_pos = egui::Pos2::new(
                        (click_pos.x - top_left.x) / zoom,
                        (click_pos.y - top_left.y) / zoom,
                    );
                    self.handle_tool_action(
                        map_pos,
                        ui.ctx(),
                        response.drag_started() || response.clicked(),
                    );
                }
            } else if response.drag_stopped_by(egui::PointerButton::Primary) {
                self.finalize_stroke();
            }

            // ブラシプレビュー表示
            if self.active_tool == ActiveTool::Brush {
                if let Some(hover_pos) = response.hover_pos() {
                    let radius_screen = self.brush_size as f32 * zoom;
                    painter.circle_stroke(
                        hover_pos,
                        radius_screen,
                        egui::Stroke::new(1.0, egui::Color32::WHITE),
                    );
                }
            }

            // 選択中のプロヴィンス表示 (重心)
            if let Some(project) = &self.project {
                for &sel_id in &self.selected_provinces {
                    if let Some(data) = project.graph.get_province(sel_id) {
                        let centroid = data.centroid();
                        let screen_pos = egui::Pos2::new(
                            top_left.x + centroid.0 * zoom,
                            top_left.y + centroid.1 * zoom,
                        );
                        painter.circle_filled(screen_pos, 4.0, egui::Color32::YELLOW);
                    }
                }
            }
        } else {
            // マップ未読み込み時の表示
            let painter = ui.painter_at(available_rect);
            painter.text(
                available_rect.center(),
                egui::Align2::CENTER_CENTER,
                "ファイル → マップフォルダを開く\nで HoI4 の map/ フォルダを選択してください",
                egui::FontId::proportional(20.0),
                egui::Color32::GRAY,
            );
        }

        // Painter オーバーレイ描画（マップの有無に関わらずビューポート基準で描画）
        let painter = ui.painter_at(available_rect);

        // Voronoi ポイント
        if self.grid_mode == GridMode::VoronoiPoints {
            for p in &self.painter_points {
                let map_pos = egui::Pos2::new(p.x, p.y);
                let screen_pos = self.viewport.map_to_screen(map_pos, available_rect);
                let radius = 4.0;
                painter.circle_stroke(
                    screen_pos,
                    radius,
                    egui::Stroke::new(1.5, egui::Color32::YELLOW),
                );
                painter.circle_filled(screen_pos, 1.5, egui::Color32::YELLOW);
            }
        }

        // Hex グリッド
        if self.grid_mode == GridMode::HexGrid {
            // ビューポート内に見える範囲だけセルを走査
            let rect = available_rect;
            let top_left_map = self.viewport.screen_to_map(rect.min, rect);
            let bottom_right_map = self.viewport.screen_to_map(rect.max, rect);

            // 粗い範囲推定
            let min_cell = map_pos_to_hex_cell(top_left_map, &self.hex_config);
            let max_cell = map_pos_to_hex_cell(bottom_right_map, &self.hex_config);

            let (min_q, max_q) = if min_cell.0 <= max_cell.0 {
                (min_cell.0 - 2, max_cell.0 + 2)
            } else {
                (max_cell.0 - 2, min_cell.0 + 2)
            };
            let (min_r, max_r) = if min_cell.1 <= max_cell.1 {
                (min_cell.1 - 2, max_cell.1 + 2)
            } else {
                (max_cell.1 - 2, min_cell.1 + 2)
            };

            for q in min_q..=max_q {
                for r in min_r..=max_r {
                    let poly = hex_cell_to_polygon((q, r), &self.hex_config);
                    let mut points_screen = Vec::with_capacity(6);
                    for mp in poly.iter() {
                        points_screen.push(self.viewport.map_to_screen(*mp, available_rect));
                    }
                    let mut lines = points_screen.clone();
                    if let Some(first) = lines.first().copied() {
                        lines.push(first);
                    }
                    painter.line(
                        lines,
                        egui::Stroke::new(
                            0.5,
                            egui::Color32::from_rgba_unmultiplied(200, 200, 255, 80),
                        ),
                    );
                }
            }
        }
    }
}

impl eframe::App for WorldSmithApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // キーボードショートカット
        ctx.input(|i| {
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Z) {
                if let Some(project) = &mut self.project {
                    let _ = self.command_stack.undo(project);
                }
            }
            if i.modifiers.ctrl && i.key_pressed(egui::Key::Y) {
                if let Some(project) = &mut self.project {
                    let _ = self.command_stack.redo(project);
                }
            }
        });

        // トップメニューバー
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            self.draw_menu_bar(ui, ctx);
        });

        // ステータスバー
        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.label(&self.status_message);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(format!("ズーム: {:.0}%", self.viewport.zoom * 100.0));
                    ui.separator();
                    if self.command_stack.can_undo() {
                        ui.label("Undo ✓");
                    }
                });
            });
        });

        // 左パネル
        egui::SidePanel::left("tools_panel")
            .default_width(180.0)
            .show(ctx, |ui| {
                self.draw_left_panel(ui);
            });

        // 右パネル
        egui::SidePanel::right("inspector_panel")
            .default_width(220.0)
            .show(ctx, |ui| {
                self.draw_right_panel(ui);
            });

        // 中央パネル (マップ)
        egui::CentralPanel::default().show(ctx, |ui| {
            self.draw_map_viewport(ui);
        });
    }
}
