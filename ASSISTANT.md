> このファイルは司令塔(assistant ワークスペース)の観測メモ。拘束力はない。
> コードや CLAUDE.md と矛盾する場合はそちらが正。参考にしてもしなくてもよい。

## このプロジェクトは何をしたい奴か

### 目的・概要
『Hearts of Iron IV (HoI4)』Mod制作に向けたベクターファーストな高機能マップ編集ツール「World Smith」（パッケージ名: `world_smith`）。
手描きのビットマップ画像（`provinces.bmp`）直塗りによる煩雑さを解消し、プロヴィンスの生成、定義ファイル（`definition.csv`）の解析・編集、範囲選択によるステート・地形属性の一括設定などを効率化する。

### 主要機能
- **`definition.csv` の一元管理**: プロヴィンスID、RGBカラー、プロヴィンス種別（Land/Sea/Lake）、沿岸判定、地形、大陸IDの相互変換・編集・フォーマット保持保存。
- **グラフ＆空間データ管理**: プロヴィンスの隣接グラフ構造化および R*-tree (`rstar`) を用いた高速な空間インデックス検索・選択処理。
- **GUI マップエディタ**: `eframe` / `egui` (wgpuレンダラー) による高速インタラクティブ画面。マップ表示・塗りつぶし・範囲選択・各種属性設定。
- **Undo / Redo コマンドシステム**: 編集操作のUndo/Redo履歴管理。
- **多言語対応 (i18n)**: `fluent` ライブラリによるGUIテキストのローカライズ対応。

### 技術構成 / スタック
- **言語**: Rust (Edition 2021)
- **GUI / レンダリング**: `eframe` (0.31, wgpu), `egui`, `egui_extras`
- **データ・画像処理**: `image` (BMP/PNG), `csv`, `serde`, `serde_json`
- **空間検索 / アルゴリズム**: `rstar` (R*-Tree 空間インデックス), `rand_xoshiro`
- **多言語対応 (i18n)**: `fluent`, `unic-langid`, `fluent-langneg`, `intl-memoizer`
- **ユーティリティ / その他**: `rfd` (ファイルダイアログ), `anyhow`, `env_logger`, `criterion` (ベンチマーク)
