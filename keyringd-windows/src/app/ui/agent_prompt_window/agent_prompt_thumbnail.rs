use windows::Win32::Foundation::COLORREF;
use windows::Win32::Graphics::Dwm::{
    DWMWA_FORCE_ICONIC_REPRESENTATION, DWMWA_HAS_ICONIC_BITMAP, DwmSetIconicThumbnail,
    DwmSetWindowAttribute,
};
use windows::Win32::Graphics::Gdi::{
    BI_RGB, BITMAPINFO, BITMAPINFOHEADER, CreateDIBSection, DIB_RGB_COLORS, DeleteObject, HGDIOBJ,
};
use windows::Win32::UI::WindowsAndMessaging::{LWA_ALPHA, SetLayeredWindowAttributes};

use windows::Win32::Foundation::HWND;

pub(super) struct AgentPromptThumbnail;

impl AgentPromptThumbnail {
    pub(super) fn configure(window: HWND) -> windows::core::Result<()> {
        let enabled = 1_i32;
        unsafe {
            DwmSetWindowAttribute(
                window,
                DWMWA_FORCE_ICONIC_REPRESENTATION,
                std::ptr::from_ref(&enabled).cast(),
                std::mem::size_of_val(&enabled) as u32,
            )?;
            DwmSetWindowAttribute(
                window,
                DWMWA_HAS_ICONIC_BITMAP,
                std::ptr::from_ref(&enabled).cast(),
                std::mem::size_of_val(&enabled) as u32,
            )?;
            SetLayeredWindowAttributes(window, COLORREF(0), 0, LWA_ALPHA)?;
        }
        Ok(())
    }

    pub(super) fn render(window: HWND, width: u32, height: u32) -> windows::core::Result<()> {
        let header = BITMAPINFOHEADER {
            biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: width as i32,
            biHeight: -(height as i32),
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB.0,
            ..Default::default()
        };
        let info = BITMAPINFO {
            bmiHeader: header,
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let bitmap = unsafe { CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)? };
        let pixels = unsafe {
            std::slice::from_raw_parts_mut(bits.cast::<u32>(), width as usize * height as usize)
        };
        Self::draw_icon(pixels, width as usize, height as usize);
        let result = unsafe { DwmSetIconicThumbnail(window, bitmap, 0) };
        unsafe {
            let _ = DeleteObject(HGDIOBJ(bitmap.0));
        }
        result
    }

    fn draw_icon(pixels: &mut [u32], width: usize, height: usize) {
        const SOURCE_SIZE: usize = 256;
        let source: &[u8; SOURCE_SIZE * SOURCE_SIZE * 4] =
            include_bytes!(concat!(env!("OUT_DIR"), "/keyringd-256.rgba"));
        let icon_size = width.min(height);
        let offset_x = (width - icon_size) / 2;
        let offset_y = (height - icon_size) / 2;
        pixels.fill(0);

        for y in 0..icon_size {
            for x in 0..icon_size {
                let source_x = x * SOURCE_SIZE / icon_size;
                let source_y = y * SOURCE_SIZE / icon_size;
                let source_offset = (source_y * SOURCE_SIZE + source_x) * 4;
                let red = u32::from(source[source_offset]);
                let green = u32::from(source[source_offset + 1]);
                let blue = u32::from(source[source_offset + 2]);
                let alpha = u32::from(source[source_offset + 3]);
                pixels[(y + offset_y) * width + x + offset_x] =
                    (alpha << 24) | (red << 16) | (green << 8) | blue;
            }
        }
    }
}
