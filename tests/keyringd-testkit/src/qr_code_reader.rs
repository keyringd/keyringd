use anyhow::{Context, Result, ensure};
use std::path::Path;

/// Reads the QR pixels captured by an OS-specific UI driver.
pub struct QrCodeReader;

impl QrCodeReader {
    /// No QR is normal while a window transitions between prompts.
    /// Multiple codes, unreadable codes and non-Hybrid payloads are errors.
    pub fn read_hybrid(path: &Path) -> Result<Option<String>> {
        let pixels = image::open(path)
            .with_context(|| format!("failed to read QR screenshot: {}", path.display()))?
            .into_luma8();
        let mut prepared = rqrr::PreparedImage::prepare(pixels);
        let grids = prepared.detect_grids();
        if grids.is_empty() {
            return Ok(None);
        }
        ensure!(grids.len() == 1, "screenshot contains multiple QR codes");
        let (_, text) = grids[0]
            .decode()
            .with_context(|| format!("failed to decode QR in {}", path.display()))?;
        ensure!(
            text.starts_with("FIDO:/") && !text.contains(['\r', '\n']),
            "screenshot QR is not a Hybrid authentication URI"
        );
        Ok(Some(text))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use image::{GrayImage, Luma};
    use qrcode::{Color, QrCode};

    fn draw(image: &mut GrayImage, text: &str, offset: u32) {
        let code = QrCode::new(text).unwrap();
        let scale = 4;
        for y in 0..code.width() {
            for x in 0..code.width() {
                if code[(x, y)] == Color::Dark {
                    for dy in 0..scale {
                        for dx in 0..scale {
                            image.put_pixel(
                                offset + (x as u32 + 4) * scale + dx,
                                (y as u32 + 4) * scale + dy,
                                Luma([0]),
                            );
                        }
                    }
                }
            }
        }
    }

    fn read(image: GrayImage) -> Result<Option<String>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("window.png");
        image.save(&path)?;
        QrCodeReader::read_hybrid(&path)
    }

    keyringd_testkit::scenario! {
        group "QrCodeReader::read_hybrid" {
            case "Reads a dense hybrid qr from a window image" {
                verify "decode the expected Hybrid URI from the rendered QR image" {
                    std::thread::spawn(|| {
                        let uri = format!("FIDO:/{}", "0123456789".repeat(28));
                        let mut image = GrayImage::from_pixel(800, 600, Luma([255]));
                        draw(&mut image, &uri, 40);
                        assert_eq!(read(image).unwrap(), Some(uri));
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "QrCodeReader::read_hybrid" {
            case "Returns none during a transition without a qr" {
                verify "return an empty scan while the transitioning window has no visible QR" {
                    std::thread::spawn(|| {
                        assert_eq!(
                            read(GrayImage::from_pixel(200, 200, Luma([255]))).unwrap(),
                            None
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }

    keyringd_testkit::scenario! {
        group "QrCodeReader::read_hybrid" {
            case "Rejects ambiguous and non hybrid images" {
                verify "reject a frame with more than one QR or a QR for another protocol" {
                    std::thread::spawn(|| {
                        let mut image = GrayImage::from_pixel(500, 200, Luma([255]));
                        draw(&mut image, "FIDO:/123456789012345678", 0);
                        draw(&mut image, "FIDO:/987654321098765432", 250);
                        assert!(read(image).unwrap_err().to_string().contains("multiple QR"));
                        let mut image = GrayImage::from_pixel(200, 200, Luma([255]));
                        draw(&mut image, "https://example.com", 0);
                        assert!(
                            read(image)
                                .unwrap_err()
                                .to_string()
                                .contains("not a Hybrid")
                        );
                    })
                    .join()
                    .unwrap();
                }
            }
        }
    }
}
