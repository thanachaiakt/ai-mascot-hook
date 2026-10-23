//! The icon menu: a row of round buttons over the seal, and a settings row it unfolds.

use super::*;

pub(super) const MENU_STEPS: u32 = 6;

// what a menu button does, posted to the seal once the menu has closed
pub(super) const ACT_STATUS: usize = 0;
pub(super) const ACT_EDIT: usize = 1;
pub(super) const ACT_RESIZE: usize = 2;
pub(super) const ACT_QUIT: usize = 3;

/// The menu's buttons: the main row, left to right, then the settings row.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Item {
    Resize,
    Settings,
    Close,
    Status,
    Edit,
}
pub(super) const MAIN_ROW: [Item; 3] = [Item::Resize, Item::Settings, Item::Close];
pub(super) const SETTINGS_ROW: [Item; 2] = [Item::Status, Item::Edit];

/// Where everything in the menu window sits right now.
pub(super) struct MenuLayout {
    pub(super) w: i32,
    pub(super) h: i32,
    pub(super) btn: i32,
    /// each visible button and its center
    pub(super) buttons: Vec<(Item, i32, i32)>,
    /// top of the label strip under the main row, and under the settings row
    pub(super) main_label: i32,
    pub(super) settings_label: i32,
}


pub(super) const MENU_PNG: &[u8] = include_bytes!("../../assets/menu.png");

// design-pixel sizes of the menu window's padding and the label strip between the main row
// and the seal; `menu_overlap` needs them to know how much of the window is empty
const MENU_PAD: f32 = 10.0;
const MENU_LABEL_GAP: f32 = 8.0;
const MENU_LABEL: f32 = 28.0;
// the space left between the buttons and the seal
const MENU_SEAL_GAP: f32 = 6.0;

impl App {
    /// How far the menu window reaches into the seal's box on the side it opens from. The
    /// label strip is empty until a button is hovered, so the window covers it with the seal
    /// and keeps the buttons themselves close.
    fn menu_overlap(&self) -> i32 {
        let design = if self.menu_below { 10.0 } else { MENU_PAD + MENU_LABEL_GAP + MENU_LABEL - MENU_SEAL_GAP };
        (design * self.ui) as i32
    }

    // ---- the icon menu ---------------------------------------------------------

    /// The menu's layout: a main row (resize, settings, close) bowed into an arc, and, while
    /// settings is open, a row of its two buttons on the side away from the seal, centered
    /// over the settings button. Each row has a label strip on its seal side.
    pub(super) fn menu_layout(&self) -> MenuLayout {
        let ui = self.ui;
        let btn = (48.0 * ui).round() as i32;
        let gap = (12.0 * ui) as i32;
        let pad = (MENU_PAD * ui) as i32;
        let raise = (14.0 * ui) as i32;
        let label_gap = (MENU_LABEL_GAP * ui) as i32;
        let label = (MENU_LABEL * ui) as i32;
        let n = MAIN_ROW.len() as i32;
        let w = n * btn + (n - 1) * gap + 2 * pad;
        let main_h = raise + btn + label_gap + label;
        let settings_h = btn + label_gap + label;
        let (main_top, settings_top) = match (self.settings_open, self.menu_below) {
            (false, _) => (pad, pad),
            // above the seal: the settings row goes on top, farthest from the seal
            (true, false) => (pad + settings_h, pad),
            // below the seal: it goes underneath
            (true, true) => (pad, pad + main_h),
        };
        let h = 2 * pad + main_h + if self.settings_open { settings_h } else { 0 };

        let mut buttons = Vec::with_capacity(MAIN_ROW.len() + SETTINGS_ROW.len());
        for (i, &item) in MAIN_ROW.iter().enumerate() {
            // an arc: the outer buttons sit lower than the middle one
            let outer = i == 0 || i == MAIN_ROW.len() - 1;
            buttons.push((item, pad + i as i32 * (btn + gap) + btn / 2, main_top + btn / 2 + if outer { raise } else { 0 }));
        }
        if self.settings_open {
            let m = SETTINGS_ROW.len() as i32;
            let left = w / 2 - (m * btn + (m - 1) * gap) / 2;
            for (i, &item) in SETTINGS_ROW.iter().enumerate() {
                buttons.push((item, left + i as i32 * (btn + gap) + btn / 2, settings_top + btn / 2));
            }
        }
        MenuLayout { w, h, btn, buttons, main_label: main_top + raise + btn + label_gap, settings_label: settings_top + btn + label_gap }
    }

    pub(super) fn menu_hit(&self, client: POINT) -> Option<Item> {
        let layout = self.menu_layout();
        let r = layout.btn as f32 * 0.42;
        layout
            .buttons
            .iter()
            .find(|&&(_, cx, cy)| (((client.x - cx).pow(2) + (client.y - cy).pow(2)) as f32).sqrt() <= r)
            .map(|&(item, _, _)| item)
    }

    pub(super) fn toggle_menu(&mut self) {
        if self.menu_open {
            self.close_menu();
        } else {
            self.open_menu();
        }
    }

    /// Put the menu window over (or under) the seal for its current size.
    pub(super) fn place_menu(&mut self) {
        let MenuLayout { w, h, .. } = self.menu_layout();
        let (mw, mh) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let mx = self.x + mw / 2 - w / 2;
        let overlap = self.menu_overlap();
        let my = if self.menu_below { self.y + mh - overlap } else { self.y - h + overlap };
        self.menu_at = (mx.clamp(work.left, (work.right - w).max(work.left)), my.clamp(work.top, (work.bottom - h).max(work.top)));
    }

    pub(super) fn open_menu(&mut self) {
        self.settings_open = false;
        self.settings_step = 0;
        let h = self.menu_layout().h;
        let (mw, _) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        self.menu_below = false;
        self.menu_below = self.y - h + self.menu_overlap() < work.top;
        self.place_menu();
        self.menu_open = true;
        self.menu_hover = None;
        self.menu_step = 0;
        self.update_bubble();
        self.draw_menu();
        unsafe {
            ShowWindow(self.menu, SW_SHOW);
            SetForegroundWindow(self.menu);
            SetTimer(self.menu, TIMER_MENU, 16, None);
        }
    }

    pub(super) fn close_menu(&mut self) {
        if self.menu_open {
            self.menu_open = false;
            self.settings_open = false;
            self.menu_hover = None;
            self.tracking_leave = false;
            unsafe {
                KillTimer(self.menu, TIMER_MENU);
                ShowWindow(self.menu, SW_HIDE);
            }
            self.update_bubble();
        }
    }

    /// Unfold or fold the settings row; the main row stays where it is on screen.
    pub(super) fn toggle_settings(&mut self) {
        self.settings_open = !self.settings_open;
        self.settings_step = 0;
        self.place_menu();
        // the window moved under a still pointer: find what it is over now
        let p = cursor_pos();
        self.menu_hover = self.menu_hit(POINT { x: p.x - self.menu_at.0, y: p.y - self.menu_at.1 });
        self.draw_menu();
        if self.settings_open {
            unsafe { SetTimer(self.menu, TIMER_MENU, 16, None) };
        }
    }

    pub(super) fn label_for(&self, item: Item) -> &'static str {
        match item {
            Item::Resize => "Resize",
            Item::Settings => "Settings",
            Item::Close => "Close",
            Item::Status => if self.show_status { "Hide status" } else { "Show status" },
            Item::Edit => "Edit status",
        }
    }

    pub(super) fn draw_menu(&mut self) {
        let layout = self.menu_layout();
        let (w, h, btn) = (layout.w, layout.h, layout.btn);
        let ui = self.ui;
        self.nc.reset(w, h);
        // the buttons pop up from the seal: they slide the last few pixels as they fade in
        let eased = |step: u32| 1.0 - (1.0 - step as f32 / MENU_STEPS as f32).powi(3);
        let ease = eased(self.menu_step);
        let sub_ease = eased(self.settings_step);
        let toward_seal = if self.menu_below { -1 } else { 1 };
        for &(item, cx, cy) in &layout.buttons {
            let hovered = self.menu_hover == Some(item);
            let base = match item {
                Item::Status => if self.show_status { 2 } else { 0 },
                Item::Edit => 4,
                Item::Resize => 6,
                Item::Close => 8,
                Item::Settings => 10,
            };
            // settings keeps its ring while its row is open
            let lit = hovered || (item == Item::Settings && self.settings_open);
            let tile = &self.buttons[base + lit as usize];
            if SETTINGS_ROW.contains(&item) {
                let slide = ((1.0 - sub_ease) * 8.0 * ui) as i32 * toward_seal;
                self.nc.blit_faded(tile, cx - btn / 2, cy - btn / 2 + slide, (255.0 * sub_ease) as u8);
            } else {
                let slide = ((1.0 - ease) * 10.0 * ui) as i32 * toward_seal;
                self.nc.blit(tile, cx - btn / 2, cy - btn / 2 + slide);
            }
        }
        if let Some(item) = self.menu_hover {
            let text = self.label_for(item);
            let center = layout.buttons.iter().find(|b| b.0 == item).map(|b| b.1).unwrap_or(w / 2);
            let label_y = if SETTINGS_ROW.contains(&item) { layout.settings_label } else { layout.main_label };
            let (tw, th) = self.nc.measure(self.font, text);
            let (pw, ph) = (tw + (20.0 * ui) as i32, th + (8.0 * ui) as i32);
            let px = (center - pw / 2).clamp(0, (w - pw).max(0));
            let (x0, y0, x1, y1) = (px as f32, label_y as f32, (px + pw) as f32, (label_y + ph) as f32);
            self.nc.fill_sdf((x0, y0, x1, y1), NAVY, 1.0, round_rect(x0, y0, x1, y1, ph as f32 / 2.0));
            self.nc.text(self.font, text, RECT { left: px, top: label_y, right: px + pw, bottom: label_y + ph }, WHITE);
        }
        let alpha = (255.0 * ease) as u8;
        self.nc.present(self.menu, self.menu_at.0, self.menu_at.1, alpha);
    }

    pub(super) fn menu_tick(&mut self) {
        let settings_done = !self.settings_open || self.settings_step >= MENU_STEPS;
        if self.menu_step < MENU_STEPS || !settings_done {
            self.menu_step = (self.menu_step + 1).min(MENU_STEPS);
            if self.settings_open {
                self.settings_step = (self.settings_step + 1).min(MENU_STEPS);
            }
            self.draw_menu();
        } else {
            unsafe { KillTimer(self.menu, TIMER_MENU) };
        }
    }

    pub(super) fn menu_move(&mut self, client: POINT) {
        if !self.tracking_leave {
            let mut track = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: self.menu, dwHoverTime: 0 };
            unsafe { TrackMouseEvent(&mut track) };
            self.tracking_leave = true;
        }
        let hover = self.menu_hit(client);
        if hover != self.menu_hover {
            self.menu_hover = hover;
            self.draw_menu();
        }
    }

    pub(super) fn menu_leave(&mut self) {
        self.tracking_leave = false;
        if self.menu_hover.is_some() {
            self.menu_hover = None;
            self.draw_menu();
        }
    }

    pub(super) fn menu_click(&mut self, client: POINT) {
        let action = match self.menu_hit(client) {
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
    pub(super) fn menu_escape(&mut self) {
        if self.settings_open {
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
                unsafe { ScreenToClient(app.menu, &mut p) };
                app.menu_hit(p).is_some()
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
