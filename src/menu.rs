//! The icon menu: a row of round buttons over the seal, and a settings row it unfolds. It is
//! laid out, drawn and animated here; each platform shows its canvas in a window of its own
//! and passes the pointer in.

use crate::bubble::Bounds;
use crate::gfx::*;

const MENU_STEPS: u32 = 6;

/// The menu's buttons: the main row, left to right, then the settings row.
#[derive(Clone, Copy, PartialEq)]
pub enum Item {
    Resize,
    Settings,
    Close,
    Status,
    Edit,
}
const MAIN_ROW: [Item; 3] = [Item::Resize, Item::Settings, Item::Close];
const SETTINGS_ROW: [Item; 2] = [Item::Status, Item::Edit];

/// Where everything in the menu window sits right now.
struct MenuLayout {
    w: i32,
    h: i32,
    btn: i32,
    /// each visible button and its center
    buttons: Vec<(Item, i32, i32)>,
    /// top of the label strip under the main row, and under the settings row
    main_label: i32,
    settings_label: i32,
}

const MENU_PNG: &[u8] = include_bytes!("../assets/menu.png");

// design-pixel sizes of the menu window's padding and the label strip between the main row
// and the seal; `overlap` needs them to know how much of the window is empty
const MENU_PAD: f32 = 10.0;
const MENU_LABEL_GAP: f32 = 8.0;
const MENU_LABEL: f32 = 28.0;
// the space left between the buttons and the seal
const MENU_SEAL_GAP: f32 = 6.0;

pub struct Menu {
    pub open: bool,
    pub hover: Option<Item>,
    step: u32,
    pub settings_open: bool,
    settings_step: u32,
    /// the menu window's top left, in screen pixels
    pub at: (i32, i32),
    /// the menu opens under the seal, there being no room above it
    below: bool,
    ui: f32,
    font: Font,
    /// each button, plain then ringed, cut from the sprite sheet at the display's size
    buttons: Vec<Image>,
    /// the menu as drawn
    pub canvas: Canvas,
}

impl Menu {
    pub fn new(font: Font, ui: f32) -> Menu {
        let sheet = Image::decode(MENU_PNG);
        let btn = (48.0 * ui).round() as usize;
        let buttons = (0..12).map(|i| sheet.tile(i, 12, btn)).collect();
        Menu { open: false, hover: None, step: 0, settings_open: false, settings_step: 0, at: (0, 0), below: false, ui, font, buttons, canvas: Canvas::new() }
    }

    /// How far the menu window reaches into the seal's box on the side it opens from. The
    /// label strip is empty until a button is hovered, so the window covers it with the seal
    /// and keeps the buttons themselves close.
    fn overlap(&self) -> i32 {
        let design = if self.below { 10.0 } else { MENU_PAD + MENU_LABEL_GAP + MENU_LABEL - MENU_SEAL_GAP };
        (design * self.ui) as i32
    }

    /// The menu's layout: a main row (resize, settings, close) bowed into an arc, and, while
    /// settings is open, a row of its two buttons on the side away from the seal, centered
    /// over the settings button. Each row has a label strip on its seal side.
    fn layout(&self) -> MenuLayout {
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
        let (main_top, settings_top) = match (self.settings_open, self.below) {
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

    /// The button under (x, y), in the menu window's pixels.
    pub fn hit(&self, x: i32, y: i32) -> Option<Item> {
        let layout = self.layout();
        let r = layout.btn as f32 * 0.42;
        layout.buttons.iter().find(|&&(_, cx, cy)| (((x - cx).pow(2) + (y - cy).pow(2)) as f32).sqrt() <= r).map(|&(item, _, _)| item)
    }

    /// Put the menu window over (or under) `seal` (x, y, w, h), inside `work`.
    fn place(&mut self, seal: (i32, i32, i32, i32), work: Bounds) {
        let MenuLayout { w, h, .. } = self.layout();
        let (sx, sy, sw, sh) = seal;
        let mx = sx + sw / 2 - w / 2;
        let overlap = self.overlap();
        let my = if self.below { sy + sh - overlap } else { sy - h + overlap };
        self.at = (mx.clamp(work.0, (work.2 - w).max(work.0)), my.clamp(work.1, (work.3 - h).max(work.1)));
    }

    /// Open over `seal`, or under it when there is no room above in `work`.
    pub fn open(&mut self, seal: (i32, i32, i32, i32), work: Bounds) {
        self.settings_open = false;
        self.settings_step = 0;
        self.below = false;
        self.below = seal.1 - self.layout().h + self.overlap() < work.1;
        self.place(seal, work);
        self.open = true;
        self.hover = None;
        self.step = 0;
    }

    /// Close; true if it was open.
    pub fn close(&mut self) -> bool {
        let was = self.open;
        self.open = false;
        self.settings_open = false;
        self.hover = None;
        was
    }

    /// Unfold or fold the settings row; the main row stays where it is on screen. `cursor`
    /// is the pointer in screen pixels: the window moves under it.
    pub fn toggle_settings(&mut self, seal: (i32, i32, i32, i32), work: Bounds, cursor: (i32, i32)) {
        self.settings_open = !self.settings_open;
        self.settings_step = 0;
        self.place(seal, work);
        self.hover = self.hit(cursor.0 - self.at.0, cursor.1 - self.at.1);
    }

    /// Note the button the pointer is over; true when that changed.
    pub fn set_hover(&mut self, hover: Option<Item>) -> bool {
        let changed = hover != self.hover;
        self.hover = hover;
        changed
    }

    fn label_for(item: Item, show_status: bool) -> &'static str {
        match item {
            Item::Resize => "Resize",
            Item::Settings => "Settings",
            Item::Close => "Close",
            Item::Status => if show_status { "Hide status" } else { "Show status" },
            Item::Edit => "Edit status",
        }
    }

    /// Draw the menu into `canvas`. Returns the window's opacity, which fades in with it.
    pub fn draw(&mut self, show_status: bool) -> u8 {
        let layout = self.layout();
        let (w, h, btn) = (layout.w, layout.h, layout.btn);
        let ui = self.ui;
        self.canvas.reset(w, h);
        // the buttons pop up from the seal: they slide the last few pixels as they fade in
        let eased = |step: u32| 1.0 - (1.0 - step as f32 / MENU_STEPS as f32).powi(3);
        let ease = eased(self.step);
        let sub_ease = eased(self.settings_step);
        let toward_seal = if self.below { -1 } else { 1 };
        for &(item, cx, cy) in &layout.buttons {
            let hovered = self.hover == Some(item);
            let base = match item {
                Item::Status => if show_status { 2 } else { 0 },
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
                self.canvas.blit_faded(tile, cx - btn / 2, cy - btn / 2 + slide, (255.0 * sub_ease) as u8);
            } else {
                let slide = ((1.0 - ease) * 10.0 * ui) as i32 * toward_seal;
                self.canvas.blit(tile, cx - btn / 2, cy - btn / 2 + slide);
            }
        }
        if let Some(item) = self.hover {
            let text = Menu::label_for(item, show_status);
            let center = layout.buttons.iter().find(|b| b.0 == item).map(|b| b.1).unwrap_or(w / 2);
            let label_y = if SETTINGS_ROW.contains(&item) { layout.settings_label } else { layout.main_label };
            let (tw, th) = self.canvas.measure(self.font, text);
            let (pw, ph) = (tw + (20.0 * ui) as i32, th + (8.0 * ui) as i32);
            let px = (center - pw / 2).clamp(0, (w - pw).max(0));
            let (x0, y0, x1, y1) = (px as f32, label_y as f32, (px + pw) as f32, (label_y + ph) as f32);
            self.canvas.fill_sdf((x0, y0, x1, y1), NAVY, 1.0, round_rect(x0, y0, x1, y1, ph as f32 / 2.0));
            self.canvas.text(self.font, text, Rect { left: px, top: label_y, right: px + pw, bottom: label_y + ph }, WHITE);
        }
        (255.0 * ease) as u8
    }

    /// Step the opening animations. True while one is still running and the menu wants
    /// drawing again.
    pub fn tick(&mut self) -> bool {
        let settings_done = !self.settings_open || self.settings_step >= MENU_STEPS;
        if self.step >= MENU_STEPS && settings_done {
            return false;
        }
        self.step = (self.step + 1).min(MENU_STEPS);
        if self.settings_open {
            self.settings_step = (self.settings_step + 1).min(MENU_STEPS);
        }
        true
    }
}
