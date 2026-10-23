//! The icon menu's panel. The menu itself (layout, drawing, animation) is `crate::menu`.

use objc2_app_kit::{NSCursor, NSTrackingArea, NSTrackingAreaOptions};

use super::*;

define_class!(
    // SAFETY: NSView has no subclassing requirements, and MenuView has no Drop.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "SealPetMenuView"]
    pub(super) struct MenuView;

    impl MenuView {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let at = event.locationInWindow();
            with(|app| app.menu_move(view_pixels(self, at, app.scale)));
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            with(|app| app.menu_leave());
        }

        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, event: &NSEvent) {
            let at = event.locationInWindow();
            // quitting ends the run loop: leave the app free first
            if with(|app| app.menu_click(view_pixels(self, at, app.scale))) == Some(true) {
                NSApplication::sharedApplication(self.mtm()).terminate(None);
            }
        }

        #[unsafe(method(keyDown:))]
        fn key_down(&self, event: &NSEvent) {
            const ESCAPE: u16 = 53;
            if event.keyCode() == ESCAPE {
                with(|app| app.menu_escape());
            }
        }
    }
);

impl MenuView {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<MenuView> {
        let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
        let view: Retained<MenuView> = unsafe { msg_send![MenuView::alloc(mtm), initWithFrame: frame] };
        // hover needs moves without a button down, wherever the app's focus is
        let options = NSTrackingAreaOptions::MouseMoved | NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveAlways | NSTrackingAreaOptions::InVisibleRect;
        let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), frame, options, Some(&view), None) };
        view.addTrackingArea(&area);
        view
    }
}

impl App {
    /// The seal's box, and the work area of the screen it is on, for placing the menu.
    fn menu_anchor(&self) -> ((i32, i32, i32, i32), Bounds) {
        let (w, h) = self.size();
        ((self.x, self.y, w, h), self.work_area(self.x + w / 2, self.y))
    }

    pub(super) fn toggle_menu(&mut self) {
        if self.menu.open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    fn open_menu(&mut self) {
        let (seal, work) = self.menu_anchor();
        self.menu.open(seal, work);
        self.menu_key = false;
        self.update_bubble();
        self.draw_menu();
        self.menu_win.makeKeyAndOrderFront(None);
    }

    pub(super) fn close_menu(&mut self) {
        if self.menu.close() {
            self.menu_win.orderOut(None);
            self.update_bubble();
        }
    }

    fn toggle_settings(&mut self) {
        let (seal, work) = self.menu_anchor();
        let cursor = self.cursor();
        self.menu.toggle_settings(seal, work, cursor);
        self.draw_menu();
    }

    fn draw_menu(&mut self) {
        let alpha = self.menu.draw(self.show_status);
        present(self.mtm, &self.menu_win, &self.menu.canvas, self.menu.at.0, self.menu.at.1, alpha, self.scale);
    }

    /// Run on every tick while the menu is open: step its animation, and close it once it
    /// has lost the keyboard to another window, as a click elsewhere does.
    pub(super) fn menu_tick(&mut self) {
        if self.menu_win.isKeyWindow() {
            self.menu_key = true;
        } else if self.menu_key {
            return self.close_menu();
        }
        if self.menu.tick() {
            self.draw_menu();
        }
    }

    fn menu_move(&mut self, at: (i32, i32)) {
        let hover = self.menu.hit(at.0, at.1);
        let cursor = if hover.is_some() { NSCursor::pointingHandCursor() } else { NSCursor::arrowCursor() };
        cursor.set();
        if self.menu.set_hover(hover) {
            self.draw_menu();
        }
    }

    fn menu_leave(&mut self) {
        NSCursor::arrowCursor().set();
        if self.menu.set_hover(None) {
            self.draw_menu();
        }
    }

    /// A click in the menu. True when it asks to quit, which the caller does with the app
    /// free.
    fn menu_click(&mut self, at: (i32, i32)) -> bool {
        let Some(item) = self.menu.hit(at.0, at.1) else { return false };
        if item == Item::Settings {
            self.toggle_settings();
            return false;
        }
        self.close_menu();
        NSCursor::arrowCursor().set();
        match item {
            Item::Resize => self.set_resize_mode(true),
            Item::Status => self.toggle_status(),
            Item::Edit => self.open_editor(),
            Item::Close => return true,
            Item::Settings => {}
        }
        false
    }

    /// Escape folds the settings row first, then closes the menu.
    fn menu_escape(&mut self) {
        if self.menu.settings_open {
            self.toggle_settings();
        } else {
            self.close_menu();
        }
    }
}
