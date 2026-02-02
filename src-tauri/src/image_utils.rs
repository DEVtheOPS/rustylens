use image::{DynamicImage, GenericImageView, ImageFormat, ImageReader};
use std::io::Cursor;
use std::path::Path;

const MAX_ICON_SIZE: u32 = 512;

/// Process an image file: resize if needed, convert to PNG, return as base64 data URI
pub fn process_cluster_icon(path: &Path) -> Result<String, String> {
    // Load the image
    let img = ImageReader::open(path)
        .map_err(|e| format!("Failed to open image: {}", e))?
        .decode()
        .map_err(|e| format!("Failed to decode image: {}", e))?;

    // Resize if necessary
    let resized = resize_if_needed(img);

    // Convert to PNG and encode as base64
    let base64_data = encode_as_png_base64(&resized)?;

    // Return as data URI
    Ok(format!("data:image/png;base64,{}", base64_data))
}

/// Resize image to fit within MAX_ICON_SIZE while maintaining aspect ratio
fn resize_if_needed(img: DynamicImage) -> DynamicImage {
    let (width, height) = img.dimensions();

    // Check if resizing is needed
    if width <= MAX_ICON_SIZE && height <= MAX_ICON_SIZE {
        return img;
    }

    // Calculate new dimensions maintaining aspect ratio
    let (new_width, new_height) = if width > height {
        let ratio = MAX_ICON_SIZE as f32 / width as f32;
        (MAX_ICON_SIZE, (height as f32 * ratio) as u32)
    } else {
        let ratio = MAX_ICON_SIZE as f32 / height as f32;
        ((width as f32 * ratio) as u32, MAX_ICON_SIZE)
    };

    img.resize(new_width, new_height, image::imageops::FilterType::Lanczos3)
}

/// Encode image as PNG and return base64 string
fn encode_as_png_base64(img: &DynamicImage) -> Result<String, String> {
    let mut buffer = Vec::new();
    let mut cursor = Cursor::new(&mut buffer);

    img.write_to(&mut cursor, ImageFormat::Png)
        .map_err(|e| format!("Failed to encode PNG: {}", e))?;

    use base64::Engine;
    Ok(base64::engine::general_purpose::STANDARD.encode(&buffer))
}

// Tauri Commands

#[tauri::command]
pub fn process_icon_file(path: String) -> Result<String, String> {
    let path = Path::new(&path);
    process_cluster_icon(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resize_large_image() {
        // Create a large test image (1024x768)
        let img = DynamicImage::new_rgb8(1024, 768);
        let resized = resize_if_needed(img);

        let (width, height) = resized.dimensions();
        assert!(width <= MAX_ICON_SIZE);
        assert!(height <= MAX_ICON_SIZE);

        // Check aspect ratio is maintained
        let expected_height = (768.0 * (MAX_ICON_SIZE as f32 / 1024.0)) as u32;
        assert_eq!(height, expected_height);
    }

    #[test]
    fn test_no_resize_small_image() {
        // Create a small test image (256x256)
        let img = DynamicImage::new_rgb8(256, 256);
        let resized = resize_if_needed(img);

        let (width, height) = resized.dimensions();
        assert_eq!(width, 256);
        assert_eq!(height, 256);
    }

    #[test]
    fn test_encode_as_png_base64() {
        let img = DynamicImage::new_rgb8(100, 100);
        let result = encode_as_png_base64(&img);

        assert!(result.is_ok());
        let base64_str = result.unwrap();

        // Base64 string should not be empty
        assert!(!base64_str.is_empty());

        // Should be valid base64
        use base64::Engine;
        assert!(base64::engine::general_purpose::STANDARD
            .decode(&base64_str)
            .is_ok());
    }
}
