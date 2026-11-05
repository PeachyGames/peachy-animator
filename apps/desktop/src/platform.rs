//! Application-local native shell integration; never used by the document/engine.
pub fn configure_shortcuts() {
    #[cfg(target_os = "macos")]
    {
        use objc2::MainThreadMarker;
        use objc2_app_kit::{NSApplication, NSEventModifierFlags};
        use objc2_foundation::NSString;
        // Winit's standard app menu consumes Cmd+H before egui receives a key.
        // Leave Hide available from the menu; reserve its shortcut for Show All.
        if let Some(mtm) = MainThreadMarker::new()
            && let Some(menu) = NSApplication::sharedApplication(mtm).mainMenu()
            && let Some(item) = menu.itemAtIndex(0)
            && let Some(app_menu) = item.submenu()
        {
            for item in app_menu.itemArray().iter() {
                if item.keyEquivalent().to_string() == "h"
                    && item.keyEquivalentModifierMask() == NSEventModifierFlags::Command
                {
                    item.setKeyEquivalent(&NSString::from_str(""));
                }
            }
        }
    }
}
