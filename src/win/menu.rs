//! The icon menu's window. The menu itself (layout, drawing, animation) is `crate::menu`.

use super::*;

// what a menu button does, posted to the seal once the menu has closed
pub(super) const ACT_STATUS: usize = 0;
pub(super) const ACT_EDIT: usize = 1;
pub(super) const ACT_RESIZE: usize = 2;
pub(super) const ACT_QUIT: usize = 3;

impl App {
    /// The seal's box, and the work area of the screen it is on, for placing the menu.
    fn menu_anchor(&self) -> ((i32, i32, i32, i32), Bounds) {
        let (w, h) = self.size();
        ((self.x, self.y, w, h), bounds(work_area(self.x + w / 2, self.y)))
    }

    pub(super) fn toggle_menu(&mut self) {
        if self.menu.open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    pub(super) fn open_menu(&mut self) {
        let (seal, work) = self.menu_anchor();
        self.menu.open(seal, work);
        self.update_bubble();
        self.draw_menu();
        unsafe {
            ShowWindow(self.menu_win, SW_SHOW);
            SetForegroundWindow(self.menu_win);
            SetTimer(self.menu_win, TIMER_MENU, 16, None);
        }
    }

    pub(super) fn close_menu(&mut self) {
        if self.menu.close() {
            self.tracking_leave = false;
            unsafe {
                KillTimer(self.menu_win, TIMER_MENU);
                ShowWindow(self.menu_win, SW_HIDE);
            }
            self.update_bubble();
        }
    }

    fn toggle_settings(&mut self) {
        let (seal, work) = self.menu_anchor();
        let p = cursor_pos();
        self.menu.toggle_settings(seal, work, (p.x, p.y));
        self.draw_menu();
        if self.menu.settings_open {
            unsafe { SetTimer(self.menu_win, TIMER_MENU, 16, None) };
        }
    }

    pub(super) fn draw_menu(&mut self) {
        let alpha = self.menu.draw(self.show_status);
        self.menu.canvas.present(self.menu_win, self.menu.at.0, self.menu.at.1, alpha);
    }

    fn menu_tick(&mut self) {
        if self.menu.tick() {
            self.draw_menu();
        } else {
            unsafe { KillTimer(self.menu_win, TIMER_MENU) };
        }
    }

    fn menu_move(&mut self, client: POINT) {
        if !self.tracking_leave {
            let mut track = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: self.menu_win, dwHoverTime: 0 };
            unsafe { TrackMouseEvent(&mut track) };
            self.tracking_leave = true;
        }
        if self.menu.set_hover(self.menu.hit(client.x, client.y)) {
            self.draw_menu();
        }
    }

    fn menu_leave(&mut self) {
        self.tracking_leave = false;
        if self.menu.set_hover(None) {
            self.draw_menu();
        }
    }

    fn menu_click(&mut self, client: POINT) {
        let action = match self.menu.hit(client.x, client.y) {
            Some(Item::Settings) => return self.toggle_settings(),
            Some(Item::Resize) => ACT_RESIZE,
            Some(Item::Close) => ACT_QUIT,
            Some(Item::Status) => ACT_STATUS,
            Some(Item::Edit) => ACT_EDIT,
            None => return,
        };
        self.close_menu();
        unsafe { PostMessageW(self.mascot, WM_APP_ACT, action, 0) };
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

pub(super) unsafe extern "system" fn menu_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_SETCURSOR => {
            let over = with(|app| {
                let mut p = cursor_pos();
                unsafe { ScreenToClient(app.menu_win, &mut p) };
                app.menu.hit(p.x, p.y).is_some()
            });
            if let Some(over) = over {
                SetCursor(LoadCursorW(null_mut(), if over { IDC_HAND } else { IDC_ARROW }));
                return 1;
            }
        }
        WM_MOUSEMOVE => {
            with(|app| app.menu_move(lparam_point(lp)));
            return 0;
        }
        WM_MOUSELEAVE => {
            with(|app| app.menu_leave());
            return 0;
        }
        WM_LBUTTONUP => {
            with(|app| app.menu_click(lparam_point(lp)));
            return 0;
        }
        WM_KEYDOWN if wp == VK_ESCAPE as usize => {
            with(|app| app.menu_escape());
            return 0;
        }
        WM_ACTIVATE if (wp & 0xFFFF) as u32 == WA_INACTIVE => {
            // closed from the queue: the window taking focus may be mid-call here
            PostMessageW(hwnd, WM_APP_CLOSE_MENU, 0, 0);
        }
        WM_APP_CLOSE_MENU => {
            with(|app| app.close_menu());
            return 0;
        }
        WM_TIMER if wp == TIMER_MENU => {
            with(|app| app.menu_tick());
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}
