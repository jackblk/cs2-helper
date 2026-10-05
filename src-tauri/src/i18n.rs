//! Text the Rust side shows itself: the tray menu and notifications. The
//! webview's strings live in `src/i18n/`; keep `LANGUAGES` in sync with
//! `LANGUAGES` in `src/i18n/index.ts`.

/// Language codes accepted for `[app] language`.
pub const LANGUAGES: [&str; 2] = ["en", "vi"];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Text {
    StatusWaiting,
    StatusRunning,
    StatusPaused,
    Pause,
    Resume,
    RunOnStartup,
    ReloadConfig,
    OpenConfigFolder,
    CancelOverlayEdit,
    ShowWindow,
    Exit,
    StartedInTray,
    StillInTray,
}

/// Unknown codes fall back to English.
pub fn tr(lang: &str, text: Text) -> &'static str {
    match lang {
        "vi" => vi(text),
        _ => en(text),
    }
}

fn en(text: Text) -> &'static str {
    match text {
        Text::StatusWaiting => "Waiting for CS2",
        Text::StatusRunning => "Running",
        Text::StatusPaused => "Paused",
        Text::Pause => "Pause",
        Text::Resume => "Resume",
        Text::RunOnStartup => "Run on startup",
        Text::ReloadConfig => "Reload config",
        Text::OpenConfigFolder => "Open config folder",
        Text::CancelOverlayEdit => "Cancel overlay edit",
        Text::ShowWindow => "Show window",
        Text::Exit => "Exit",
        Text::StartedInTray => "Started in the tray. Click the tray icon to open.",
        Text::StillInTray => "Still running in the tray. Right-click the tray icon to exit.",
    }
}

fn vi(text: Text) -> &'static str {
    match text {
        Text::StatusWaiting => "Đang chờ CS2",
        Text::StatusRunning => "Đang chạy",
        Text::StatusPaused => "Tạm dừng",
        Text::Pause => "Tạm dừng",
        Text::Resume => "Tiếp tục",
        Text::RunOnStartup => "Chạy khi khởi động",
        Text::ReloadConfig => "Tải lại cấu hình",
        Text::OpenConfigFolder => "Mở thư mục cấu hình",
        Text::CancelOverlayEdit => "Hủy chỉnh lớp phủ",
        Text::ShowWindow => "Hiện cửa sổ",
        Text::Exit => "Thoát",
        Text::StartedInTray => {
            "Đã khởi động trong khay hệ thống. Nhấn vào biểu tượng ở khay để mở."
        }
        Text::StillInTray => {
            "Vẫn đang chạy trong khay hệ thống. Nhấp chuột phải vào biểu tượng ở khay để thoát."
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_language_has_its_own_text_and_unknown_falls_back_to_english() {
        assert_eq!(tr("en", Text::Exit), "Exit");
        assert_eq!(tr("vi", Text::Exit), "Thoát");
        assert_eq!(tr("xx", Text::Exit), "Exit");
    }
}
