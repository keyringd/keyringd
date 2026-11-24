use anyhow::{Context, Result, bail, ensure};
use image::RgbImage;
use std::{
    path::Path,
    time::{Duration, Instant},
};
use windows::Win32::Graphics::Gdi::ClientToScreen;
use windows::{
    Graphics::{
        Capture::{
            Direct3D11CaptureFrame, Direct3D11CaptureFramePool, GraphicsCaptureItem,
            GraphicsCaptureSession,
        },
        DirectX::{Direct3D11::IDirect3DDevice, DirectXPixelFormat},
        SizeInt32,
    },
    Win32::{
        Foundation::{HWND, POINT, RECT},
        Graphics::{
            Direct3D::D3D_DRIVER_TYPE_HARDWARE,
            Direct3D11::*,
            Dwm::{DWMWA_EXTENDED_FRAME_BOUNDS, DwmGetWindowAttribute},
            Dxgi::IDXGIDevice,
        },
        System::WinRT::{
            Direct3D11::{CreateDirect3D11DeviceFromDXGIDevice, IDirect3DDxgiInterfaceAccess},
            Graphics::Capture::IGraphicsCaptureItemInterop,
        },
        UI::WindowsAndMessaging::{GetClientRect, IsWindowVisible},
    },
    core::{Interface, factory},
};

const SETTLE: Duration = Duration::from_millis(350);
const TIMEOUT: Duration = Duration::from_secs(15);

/// Captures compositor frames for one HWND, never the surrounding desktop.
pub(crate) struct WindowCapture {
    window: HWND,
    device: ID3D11Device,
    context: ID3D11DeviceContext,
    graphics_device: IDirect3DDevice,
    item: GraphicsCaptureItem,
    pool: Direct3D11CaptureFramePool,
    session: GraphicsCaptureSession,
    size: SizeInt32,
    _style: super::WindowCaptureStyle,
}

impl WindowCapture {
    pub(crate) fn new(window: HWND) -> Result<Self> {
        let style = super::WindowCaptureStyle::enter(window)?;
        ensure!(
            GraphicsCaptureSession::IsSupported()?,
            "Windows.Graphics.Capture is unavailable"
        );
        let mut device = None;
        let mut context = None;
        unsafe {
            D3D11CreateDevice(
                None,
                D3D_DRIVER_TYPE_HARDWARE,
                Default::default(),
                D3D11_CREATE_DEVICE_BGRA_SUPPORT,
                None,
                D3D11_SDK_VERSION,
                Some(&mut device),
                None,
                Some(&mut context),
            )?;
        }
        let device = device.context("D3D11 device was not created")?;
        let context = context.context("D3D11 context was not created")?;
        let dxgi: IDXGIDevice = device.cast()?;
        let graphics_device: IDirect3DDevice =
            unsafe { CreateDirect3D11DeviceFromDXGIDevice(&dxgi)? }.cast()?;
        let interop: IGraphicsCaptureItemInterop =
            factory::<GraphicsCaptureItem, IGraphicsCaptureItemInterop>()?;
        let item: GraphicsCaptureItem = unsafe { interop.CreateForWindow(window) }
            .context("could not create the window capture item")?;
        let size = item.Size()?;
        ensure!(
            size.Width > 0 && size.Height > 0,
            "capture window has no bounds"
        );
        let (pool, session) = Self::start_capture(&graphics_device, &item, size)?;
        Ok(Self {
            window,
            device,
            context,
            graphics_device,
            item,
            pool,
            session,
            size,
            _style: style,
        })
    }

    fn start_capture(
        device: &IDirect3DDevice,
        item: &GraphicsCaptureItem,
        size: SizeInt32,
    ) -> Result<(Direct3D11CaptureFramePool, GraphicsCaptureSession)> {
        let pool = Direct3D11CaptureFramePool::CreateFreeThreaded(
            device,
            DirectXPixelFormat::B8G8R8A8UIntNormalized,
            2,
            size,
        )?;
        let session = (|| {
            let session = pool.CreateCaptureSession(item)?;
            if let Err(error) = session
                .SetIsCursorCaptureEnabled(false)
                .and_then(|()| session.StartCapture())
            {
                let _ = session.Close();
                return Err(error);
            }
            Ok(session)
        })();
        match session {
            Ok(session) => Ok((pool, session)),
            Err(error) => {
                let _ = pool.Close();
                Err(error.into())
            }
        }
    }

    pub(crate) fn save_when_ready(
        &mut self,
        path: &Path,
        mut ui_state: impl FnMut() -> Result<Vec<String>>,
    ) -> Result<()> {
        let started = Instant::now();
        let mut settled = Instant::now();
        let mut state = ui_state()?;
        let mut previous: Option<RgbImage> = None;
        let mut latest: Option<RgbImage> = None;
        let mut frames = 0;
        let mut clipped_frames = 0;
        let mut ui_changes = 0;
        let mut image_changes = 0;
        while started.elapsed() < TIMEOUT {
            ensure!(
                unsafe { IsWindowVisible(self.window) }.as_bool(),
                "capture window disappeared"
            );
            let current_state = ui_state()?;
            if current_state != state {
                ui_changes += 1;
                state = current_state;
                settled = Instant::now();
                // Accessibility metadata can change without a repaint. WGC only
                // supplies changed frames, so retain the last image and restart
                // the quiet period; any actual pixel change restarts it too.
            }
            // Static windows may produce no new frames. Prefer the newest of the
            // two buffered frames, but bound draining even for animated windows.
            let mut newest: Option<Direct3D11CaptureFrame> = None;
            for _ in 0..2 {
                let frame = match self.pool.TryGetNextFrame() {
                    Ok(frame) => frame,
                    // windows-rs represents WinRT's null (no frame yet) with
                    // Error::empty. Actual capture errors must not certify an old frame.
                    Err(error) if error.code() == windows::core::Error::empty().code() => break,
                    Err(error) => {
                        return Err(error).context("could not acquire a compositor frame");
                    }
                };
                if let Some(old) = newest.replace(frame) {
                    old.Close()?;
                }
            }
            if let Some(frame) = newest {
                frames += 1;
                let size = frame.ContentSize()?;
                let pixels = if size == self.size {
                    self.read_frame(&frame)
                } else {
                    Ok(None)
                };
                frame.Close()?;
                let Some(pixels) = pixels? else {
                    if size == self.size {
                        clipped_frames += 1;
                    }
                    ensure!(
                        size.Width > 0 && size.Height > 0,
                        "capture window has no content"
                    );
                    // A static dialog may not repaint after Recreate. Start a
                    // fresh session to get its initial frame at the new size.
                    self.session.Close().context("could not stop capture")?;
                    self.pool
                        .Close()
                        .context("could not close capture buffers")?;
                    (self.pool, self.session) =
                        Self::start_capture(&self.graphics_device, &self.item, size)
                            .context("could not restart capture at the current size")?;
                    self.size = size;
                    previous = None;
                    latest = None;
                    settled = Instant::now();
                    std::thread::sleep(Duration::from_millis(30));
                    continue;
                };
                if previous
                    .as_ref()
                    .is_none_or(|old| !Self::visually_stable(old, &pixels))
                {
                    image_changes += 1;
                    settled = Instant::now();
                    previous = Some(pixels.clone());
                }
                latest = Some(pixels);
            }
            if let Some(image) = &latest
                && !state.is_empty()
                && self.client_has_content(image)?
                && settled.elapsed() >= SETTLE
            {
                image.save(path)?;
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        let diagnostic = path.with_extension("unsettled.png");
        if let Some(image) = &latest {
            image.save(&diagnostic)?;
        }
        bail!(
            "window did not reach a rendered, stable state within {TIMEOUT:?}; UI elements: {}; frames: {frames}; clipped frames: {clipped_frames}; UI changes: {ui_changes}; image changes: {image_changes}; minimized: {}; last frame: {}",
            state.len(),
            unsafe { windows::Win32::UI::WindowsAndMessaging::IsIconic(self.window) }.as_bool(),
            diagnostic.display()
        )
    }

    fn read_frame(&self, frame: &Direct3D11CaptureFrame) -> Result<Option<RgbImage>> {
        let access: IDirect3DDxgiInterfaceAccess = frame.Surface()?.cast()?;
        let texture: ID3D11Texture2D = unsafe { access.GetInterface()? };
        let mut desc = D3D11_TEXTURE2D_DESC::default();
        unsafe { texture.GetDesc(&mut desc) };
        if desc.Width < self.size.Width as u32 || desc.Height < self.size.Height as u32 {
            return Ok(None);
        }
        desc.Usage = D3D11_USAGE_STAGING;
        desc.BindFlags = 0;
        desc.CPUAccessFlags = D3D11_CPU_ACCESS_READ.0 as u32;
        desc.MiscFlags = 0;
        let mut staging = None;
        unsafe {
            self.device
                .CreateTexture2D(&desc, None, Some(&mut staging))?
        };
        let staging = staging.context("capture staging texture was not created")?;
        let mut mapped = D3D11_MAPPED_SUBRESOURCE::default();
        unsafe {
            self.context.CopyResource(&staging, &texture);
            self.context
                .Map(&staging, 0, D3D11_MAP_READ, 0, Some(&mut mapped))?;
        }
        let image = RgbImage::from_fn(self.size.Width as u32, self.size.Height as u32, |x, y| {
            let pixel = unsafe {
                std::slice::from_raw_parts(
                    mapped
                        .pData
                        .cast::<u8>()
                        .add(y as usize * mapped.RowPitch as usize + x as usize * 4),
                    4,
                )
            };
            image::Rgb([pixel[2], pixel[1], pixel[0]])
        });
        unsafe { self.context.Unmap(&staging, 0) };
        Ok(Some(image))
    }

    fn client_has_content(&self, image: &RgbImage) -> Result<bool> {
        let mut bounds = RECT::default();
        let mut client = RECT::default();
        let mut origin = POINT::default();
        unsafe {
            DwmGetWindowAttribute(
                self.window,
                DWMWA_EXTENDED_FRAME_BOUNDS,
                (&mut bounds as *mut RECT).cast(),
                size_of::<RECT>() as u32,
            )?;
            GetClientRect(self.window, &mut client)?;
            ClientToScreen(self.window, &mut origin).ok()?;
        }
        let left = (origin.x - bounds.left + 4).max(0) as u32;
        let top = (origin.y - bounds.top + 4).max(0) as u32;
        let right = (origin.x - bounds.left + client.right - 4).max(0) as u32;
        let bottom = (origin.y - bounds.top + client.bottom - 4).max(0) as u32;
        let mut minimum: u16 = 255;
        let mut maximum: u16 = 0;
        for y in top..bottom.min(image.height()) {
            for x in left..right.min(image.width()) {
                let p = image.get_pixel(x, y);
                let luminance = (u16::from(p[0]) + u16::from(p[1]) + u16::from(p[2])) / 3;
                minimum = minimum.min(luminance);
                maximum = maximum.max(luminance);
            }
        }
        Ok(maximum.saturating_sub(minimum) >= 24)
    }

    fn visually_stable(previous: &RgbImage, current: &RgbImage) -> bool {
        if previous.dimensions() != current.dimensions() {
            return false;
        }
        let difference: u64 = previous
            .as_raw()
            .iter()
            .zip(current.as_raw())
            .map(|(a, b)| u64::from(a.abs_diff(*b)))
            .sum();
        // Ignore small caret/spinner updates. Compare against the last significant
        // change, so a slow fade cannot accumulate unnoticed.
        difference <= previous.as_raw().len() as u64 / 2
    }
}

impl Drop for WindowCapture {
    fn drop(&mut self) {
        let _ = self.session.Close();
        let _ = self.pool.Close();
    }
}
