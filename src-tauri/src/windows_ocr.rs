use crate::error::{AppError, AppResult};
use crate::models::OverlayGeometry;
use std::ffi::c_void;
use std::ptr::copy_nonoverlapping;
use windows::core::Interface;
use windows::Graphics::Imaging::{BitmapAlphaMode, BitmapPixelFormat, SoftwareBitmap};
use windows::Media::Ocr::OcrEngine;
use windows::Storage::Streams::Buffer;
use windows::Win32::Graphics::Gdi::{
    BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, DeleteDC, DeleteObject, GetDC, GetDIBits,
    ReleaseDC, SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
    HGDIOBJ, SRCCOPY,
};
use windows::Win32::System::WinRT::IBufferByteAccess;

pub async fn capture_and_recognize(geometry: &OverlayGeometry) -> AppResult<String> {
    let geometry = geometry.clone();
    tauri::async_runtime::spawn_blocking(move || capture_and_recognize_blocking(&geometry))
        .await
        .map_err(|err| AppError::new("overlay_ocr_join_error", err.to_string()))?
}

fn capture_and_recognize_blocking(geometry: &OverlayGeometry) -> AppResult<String> {
    let (pixels, width, height) = capture_bgra(geometry)?;

    let bitmap = {
        let buffer = Buffer::Create(pixels.len() as u32).map_err(winrt_ocr_error)?;
        buffer
            .SetLength(pixels.len() as u32)
            .map_err(winrt_ocr_error)?;
        {
            let byte_access: IBufferByteAccess = buffer.cast().map_err(winrt_ocr_error)?;
            unsafe {
                let destination = byte_access.Buffer().map_err(winrt_ocr_error)?;
                copy_nonoverlapping(pixels.as_ptr(), destination, pixels.len());
            }
        }

        let bitmap = SoftwareBitmap::CreateWithAlpha(
            BitmapPixelFormat::Bgra8,
            width,
            height,
            BitmapAlphaMode::Premultiplied,
        )
        .map_err(winrt_ocr_error)?;
        bitmap.CopyFromBuffer(&buffer).map_err(winrt_ocr_error)?;
        bitmap
    };

    let engine = OcrEngine::TryCreateFromUserProfileLanguages().map_err(|error| {
        AppError::new(
            "windows_ocr_unavailable",
            format!(
                "Windows OCR is unavailable. Install a Windows language pack with OCR support: {error}"
            ),
        )
    })?;
    let result = engine
        .RecognizeAsync(&bitmap)
        .map_err(winrt_ocr_error)?
        .get()
        .map_err(winrt_ocr_error)?;
    result
        .Text()
        .map(|text| text.to_string())
        .map_err(winrt_ocr_error)
}

fn capture_bgra(geometry: &OverlayGeometry) -> AppResult<(Vec<u8>, i32, i32)> {
    // Tauri's innerPosition/innerSize APIs already report physical pixels. Applying
    // scale_factor again would double-scale captures on high-DPI displays.
    let x = geometry.x.round() as i32;
    let y = geometry.y.round() as i32;
    let width = geometry.width.round().max(1.0) as i32;
    let height = geometry.height.round().max(1.0) as i32;

    unsafe {
        let screen_dc = GetDC(None);
        if screen_dc.is_invalid() {
            return Err(capture_error(
                "Windows could not open the desktop device context.",
            ));
        }
        let memory_dc = CreateCompatibleDC(Some(screen_dc));
        if memory_dc.is_invalid() {
            ReleaseDC(None, screen_dc);
            return Err(capture_error(
                "Windows could not create the capture context.",
            ));
        }
        let bitmap = CreateCompatibleBitmap(screen_dc, width, height);
        if bitmap.is_invalid() {
            let _ = DeleteDC(memory_dc);
            ReleaseDC(None, screen_dc);
            return Err(capture_error(
                "Windows could not allocate the capture bitmap.",
            ));
        }

        let previous = SelectObject(memory_dc, HGDIOBJ(bitmap.0));
        let capture_result = BitBlt(
            memory_dc,
            0,
            0,
            width,
            height,
            Some(screen_dc),
            x,
            y,
            SRCCOPY | CAPTUREBLT,
        );

        let mut info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: width,
                biHeight: -height,
                biPlanes: 1,
                biBitCount: 32,
                biCompression: BI_RGB.0,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut pixels = vec![0u8; width as usize * height as usize * 4];
        let copied = if capture_result.is_ok() {
            GetDIBits(
                memory_dc,
                bitmap,
                0,
                height as u32,
                Some(pixels.as_mut_ptr().cast::<c_void>()),
                &mut info,
                DIB_RGB_COLORS,
            )
        } else {
            0
        };

        SelectObject(memory_dc, previous);
        let _ = DeleteObject(HGDIOBJ(bitmap.0));
        let _ = DeleteDC(memory_dc);
        ReleaseDC(None, screen_dc);

        if copied == 0 {
            return Err(capture_error(
                "Windows could not capture the selected desktop region.",
            ));
        }
        Ok((pixels, width, height))
    }
}

fn capture_error(message: &str) -> AppError {
    AppError::new("windows_capture_error", message)
}

fn winrt_ocr_error(error: windows::core::Error) -> AppError {
    AppError::new("windows_ocr_error", error.to_string())
}
