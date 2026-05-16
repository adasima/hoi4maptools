/// BMP マップファイルの読み込みとエクスポート。
/// provinces.bmp の RGB 値を完全に正確に読み書きする。

use anyhow::{Context, Result, bail};
use image::{ImageBuffer, RgbImage};
use std::path::Path;

/// provinces.bmp を読み込み、RGB ピクセルデータを返す。
/// 返り値: (width, height, pixels) where pixels は RGB 順の u8 配列。
pub fn load_provinces_bmp(path: &Path) -> Result<(u32, u32, Vec<u8>)> {
    log::info!("provinces.bmp を読み込み中: {}", path.display());

    let img = image::open(path)
        .with_context(|| format!("画像ファイルのオープンに失敗: {}", path.display()))?;

    let rgb_img: RgbImage = img.to_rgb8();
    let width = rgb_img.width();
    let height = rgb_img.height();
    let pixels = rgb_img.into_raw();

    log::info!(
        "provinces.bmp 読み込み完了: {}x{} ({} ピクセル)",
        width,
        height,
        width as u64 * height as u64
    );

    Ok((width, height, pixels))
}

/// RGB ピクセルデータを egui::ColorImage に変換する。
pub fn pixels_to_color_image(pixels: &[u8], width: u32, height: u32) -> egui::ColorImage {
    let size = [width as usize, height as usize];
    let rgba_pixels: Vec<egui::Color32> = pixels
        .chunks_exact(3)
        .map(|chunk| egui::Color32::from_rgb(chunk[0], chunk[1], chunk[2]))
        .collect();

    egui::ColorImage {
        size,
        pixels: rgba_pixels,
    }
}

/// RGB ピクセルデータを provinces.bmp として書き出す (デバッグエクスポート)。
/// 色の劣化を防ぐため、非圧縮 BMP で保存する。
pub fn save_provinces_bmp(path: &Path, pixels: &[u8], width: u32, height: u32) -> Result<()> {
    log::info!("provinces.bmp を書き出し中: {}", path.display());

    if pixels.len() != (width * height * 3) as usize {
        bail!(
            "ピクセルデータのサイズが不正: expected={}, actual={}",
            width * height * 3,
            pixels.len()
        );
    }

    let img: RgbImage = ImageBuffer::from_raw(width, height, pixels.to_vec())
        .context("ImageBuffer の作成に失敗")?;

    img.save(path)
        .with_context(|| format!("BMP ファイルの保存に失敗: {}", path.display()))?;

    log::info!("provinces.bmp 書き出し完了: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_roundtrip_bmp() {
        // 2x2 のテスト画像を作成
        let width = 2u32;
        let height = 2u32;
        let pixels: Vec<u8> = vec![
            255, 0, 0,     0, 255, 0,   // 赤, 緑
            0, 0, 255,   128, 64, 32,   // 青, 茶
        ];

        let tmp_path = std::env::temp_dir().join("ws_test_roundtrip.bmp");

        // 書き出し
        save_provinces_bmp(&tmp_path, &pixels, width, height).unwrap();

        // 読み込み
        let (w2, h2, pixels2) = load_provinces_bmp(&tmp_path).unwrap();

        assert_eq!(w2, width);
        assert_eq!(h2, height);
        // RGB値が完全に一致することを確認 (ロスレス)
        assert_eq!(pixels, pixels2, "BMP ラウンドトリップで RGB 値が劣化しました！");

        let _ = std::fs::remove_file(&tmp_path);
    }

    #[test]
    fn test_pixels_to_color_image() {
        let pixels: Vec<u8> = vec![255, 0, 0, 0, 255, 0];
        let img = pixels_to_color_image(&pixels, 2, 1);
        assert_eq!(img.size, [2, 1]);
        assert_eq!(img.pixels[0], egui::Color32::from_rgb(255, 0, 0));
        assert_eq!(img.pixels[1], egui::Color32::from_rgb(0, 255, 0));
    }
}
