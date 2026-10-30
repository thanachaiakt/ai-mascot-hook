//! The status editor's window and its text boxes. What it looks like and what its clicks
//! mean is `crate::editor`.

use super::*;

// the editor's controls
pub(super) const ID_EDIT0: i32 = 100;

/// The open status-text editor: its window, its text boxes and the choices being made.
pub(super) struct Editor {
    pub(super) hwnd: HWND,
    pub(super) edits: Vec<HWND>,
    pub(super) draft: Draft,
    pub(super) tracking: bool,
    pub(super) canvas: Canvas,
    pub(super) fonts: EditorFonts,
    /// the text boxes' background, the same as the field painted around them
    pub(super) field_brush: HBRUSH,
}

impl App {
    // ---- the status-text editor -------------------------------------------------

    pub(super) fn open_editor(&mut self) {
        if let Some(e) = &self.editor {
            unsafe { SetForegroundWindow(e.hwnd) };
            return;
        }
        let ui = self.ui;
        let layout = editor_layout(ui);
        let (ww, wh) = (layout.w, layout.h);
        let s = |v: f32| (v * ui).round() as i32;
        let (mw, _) = self.size();
        let work = work_area(self.x + mw / 2, self.y);
        let left_of_seal = self.x - ww - s(12.0);
        let wx = if left_of_seal >= work.left { left_of_seal } else { (self.x + mw + s(12.0)).min(work.right - ww) };
        let wy = (self.y - wh / 3).clamp(work.top, (work.bottom - wh).max(work.top));

        unsafe {
            let instance = GetModuleHandleW(null());
            let hwnd = CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                wide("SealPetEditor").as_ptr(),
                wide("ai-mascot-hook · Settings").as_ptr(),
                WS_POPUP | WS_CLIPCHILDREN,
                wx,
                wy,
                ww,
                wh,
                null_mut(),
                null_mut(),
                instance,
                null(),
            );
            // rounded corners and the system's thin border on Windows 11; ignored elsewhere
            let round = DWMWCP_ROUND;
            DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE as u32, &round as *const _ as *const _, 4);
            let fonts = EditorFonts {
                label: font(14.0 * ui, 400, "Segoe UI"),
                heading: font(11.5 * ui, 700, "Segoe UI"),
                hint: font(12.0 * ui, 400, "Segoe UI"),
                button: font(14.0 * ui, 600, "Segoe UI"),
                edit: font(14.5 * ui, 400, "Leelawadee UI"),
            };
            let canvas = Canvas::new();
            // each text box sits borderless inside its painted field, centered on one line
            let line = canvas.measure(fonts.edit, "Ag").1 + 2;
            let inset = s(12.0);
            // the states' boxes, then the break's and the feed's, each shown as the draft
            // says (the editor opens on the states' tab)
            let draft = Draft::new(&self.status, self.show_status, &self.state);
            let words = self.status.texts.iter().cloned().chain(self.status.extras());
            let mut edits = Vec::new();
            for (i, (f, words)) in layout.fields.iter().chain(&layout.extra_fields).zip(words).enumerate() {
                let top = ((f.1 + f.3) / 2.0) as i32 - line / 2;
                let visible = if draft.shows(i) { WS_VISIBLE } else { 0 };
                let edit = CreateWindowExW(
                    0,
                    wide("EDIT").as_ptr(),
                    wide(&words).as_ptr(),
                    WS_CHILD | visible | WS_TABSTOP | ES_AUTOHSCROLL as u32,
                    f.0 as i32 + inset,
                    top,
                    (f.2 - f.0) as i32 - 2 * inset,
                    line,
                    hwnd,
                    (ID_EDIT0 + i as i32) as isize as HMENU,
                    instance,
                    null(),
                );
                SendMessageW(edit, WM_SETFONT, fonts.edit as usize, 1);
                // a state's box has room for a set's ten lines
                SendMessageW(edit, EM_LIMITTEXT, if i < EXTRAS { MAX_CHARS } else { 60 }, 0);
                edits.push(edit);
            }
            self.editor = Some(Editor {
                hwnd,
                edits,
                draft,
                tracking: false,
                canvas,
                fonts,
                field_brush: CreateSolidBrush(colorref(FIELD)),
            });
            EDITOR.with(|e| e.set(hwnd));
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            let e = self.editor.as_ref().unwrap();
            PostMessageW(hwnd, WM_APP_FOCUS, e.draft.first_box(), 0);
        }
        self.update_bubble();
    }

    pub(super) fn paint_editor(&mut self, hdc: HDC) {
        let ui = self.ui;
        let Some(e) = self.editor.as_mut() else { return };
        e.draft.paint(&mut e.canvas, &e.fonts, ui);
        e.canvas.paint_to(hdc, 0, 0);
    }

    /// What in the editor is under `client`, if anything there takes a click.
    pub(super) fn editor_hit(&self, client: POINT) -> Option<Hit> {
        self.editor.as_ref()?.draft.hit(self.ui, client.x as f32, client.y as f32)
    }

    /// Show the boxes the draft shows and hide the rest, and hand the keyboard to the
    /// first shown, so typing never goes into a hidden box.
    fn sync_boxes(&self) {
        let Some(e) = self.editor.as_ref() else { return };
        unsafe {
            for (i, edit) in e.edits.iter().enumerate() {
                ShowWindow(*edit, if e.draft.shows(i) { SW_SHOW } else { SW_HIDE });
            }
            PostMessageW(e.hwnd, WM_APP_FOCUS, e.draft.first_box(), 0);
        }
    }

    /// Note what the pointer is over, repainting when that changes.
    pub(super) fn editor_hover(&mut self, hit: Option<Hit>) {
        let Some(e) = self.editor.as_mut() else { return };
        if hit.is_some() && !e.tracking {
            let mut tme = TRACKMOUSEEVENT { cbSize: std::mem::size_of::<TRACKMOUSEEVENT>() as u32, dwFlags: TME_LEAVE, hwndTrack: e.hwnd, dwHoverTime: 0 };
            unsafe { TrackMouseEvent(&mut tme) };
            e.tracking = true;
        }
        if hit.is_none() {
            e.tracking = false;
        }
        if e.draft.hover != hit {
            e.draft.hover = hit;
            unsafe { InvalidateRect(e.hwnd, null(), 0) };
        }
    }

    pub(super) fn editor_focus(&mut self, focus: Option<usize>) {
        let Some(e) = self.editor.as_mut() else { return };
        let moved = e.draft.focus_on(focus);
        unsafe { InvalidateRect(e.hwnd, null(), 0) };
        if moved {
            self.sync_clip();
            self.update_bubble();
        }
    }

    /// Do what a click on `hit` means. The color picker is left to the caller, since its
    /// dialog runs a message loop of its own.
    pub(super) fn editor_act(&mut self, hit: Hit) {
        match hit {
            Hit::Close | Hit::Cancel => return self.close_editor(false),
            Hit::Save => return self.close_editor(true),
            Hit::Reset => return self.reset_editor(),
            Hit::Swatch(i) => return self.pick_color(SWATCHES[i]),
            Hit::State(i) => {
                if let Some(e) = &self.editor {
                    unsafe { PostMessageW(e.hwnd, WM_APP_FOCUS, i, 0) };
                }
                return;
            }
            Hit::Set(set) => return self.fill_editor(set),
            _ => {}
        }
        let Some(e) = self.editor.as_mut() else { return };
        e.draft.act(hit);
        unsafe { InvalidateRect(e.hwnd, null(), 0) };
        if matches!(hit, Hit::Tab(_)) {
            self.sync_boxes();
        }
    }

    /// Color the selected state's words.
    pub(super) fn pick_color(&mut self, c: Rgba) {
        if let Some(e) = self.editor.as_mut() {
            e.draft.colors[e.draft.selected] = c;
            unsafe {
                InvalidateRect(e.hwnd, null(), 0);
                InvalidateRect(e.edits[e.draft.selected], null(), 1);
            }
        }
        self.update_bubble();
    }

    pub(super) fn reset_editor(&mut self) {
        let defaults = StatusText::defaults();
        if let Some(e) = self.editor.as_mut() {
            let words = defaults.texts.iter().cloned().chain(defaults.extras());
            for (edit, text) in e.edits.iter().zip(words) {
                unsafe { SetWindowTextW(*edit, wide(&text).as_ptr()) };
            }
            e.draft.colors = defaults.colors;
            unsafe { InvalidateRect(e.hwnd, null(), 1) };
        }
        self.update_bubble();
    }

    /// Fill every state's box with message set `set`'s lines. Like any typing, it is kept
    /// only on Save.
    pub(super) fn fill_editor(&mut self, set: usize) {
        if let Some(e) = &self.editor {
            for (i, edit) in e.edits.iter().take(EXTRAS).enumerate() {
                unsafe { SetWindowTextW(*edit, wide(&fill(set, i)).as_ptr()) };
            }
        }
        self.update_bubble();
    }

    pub(super) fn close_editor(&mut self, keep: bool) {
        let Some(e) = self.editor.take() else { return };
        if keep {
            let extras = std::array::from_fn(|i| window_text(e.edits[EXTRAS + i]));
            e.draft.commit(&mut self.status, std::array::from_fn(|i| window_text(e.edits[i])), extras);
            self.show_status = e.draft.show;
            let _ = fs::create_dir_all(data_dir());
            self.status.save(&data_dir().join("status.txt"));
            self.save();
        }
        EDITOR.with(|h| h.set(null_mut()));
        unsafe {
            DestroyWindow(e.hwnd);
            for f in [e.fonts.label, e.fonts.heading, e.fonts.hint, e.fonts.button, e.fonts.edit] {
                DeleteObject(f);
            }
            DeleteObject(e.field_brush);
        }
        self.sync_clip();
        self.restart_autohide();
    }
}

/// A custom color from the system color dialog, outside any borrow of the app: the
/// dialog runs its own message loop while it is open.
pub(super) unsafe fn choose_custom_color(owner: HWND) {
    static mut CUSTOM: [u32; 16] = [0x00FFFFFF; 16];
    let Some(current) = with(|app| app.editor.as_ref().map(|e| e.draft.colors[e.draft.selected])).flatten() else { return };
    let mut cc: CHOOSECOLORW = std::mem::zeroed();
    cc.lStructSize = std::mem::size_of::<CHOOSECOLORW>() as u32;
    cc.hwndOwner = owner;
    cc.rgbResult = colorref(current);
    cc.lpCustColors = std::ptr::addr_of_mut!(CUSTOM) as *mut u32;
    cc.Flags = CC_RGBINIT | CC_FULLOPEN;
    if ChooseColorW(&mut cc) != 0 {
        let c = cc.rgbResult;
        with(|app| app.pick_color(Rgba(c as u8, (c >> 8) as u8, (c >> 16) as u8, 255)));
    }
}

pub(super) unsafe extern "system" fn editor_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_PAINT => {
            let mut ps: PAINTSTRUCT = std::mem::zeroed();
            let hdc = BeginPaint(hwnd, &mut ps);
            with(|app| app.paint_editor(hdc));
            EndPaint(hwnd, &ps);
            return 0;
        }
        WM_ERASEBKGND => return 1,
        WM_APP_FOCUS => {
            let edit = GetDlgItem(hwnd, ID_EDIT0 + wp as i32);
            SetFocus(edit);
            SendMessageW(edit, EM_SETSEL, 0, -1isize as LPARAM);
            return 0;
        }
        // each text box shows its words in the color they will have
        WM_CTLCOLOREDIT => {
            let edit = lp as HWND;
            let found = with(|app| {
                app.editor.as_ref().map(|e| {
                    // the boxes past the states' keep the plain ink
                    let ink = e.edits.iter().position(|h| *h == edit).and_then(|i| e.draft.colors.get(i).copied()).unwrap_or(INK);
                    (e.field_brush, ink)
                })
            });
            if let Some(Some((brush, ink))) = found {
                SetBkColor(wp as HDC, colorref(FIELD));
                SetTextColor(wp as HDC, colorref(ink));
                return brush as LRESULT;
            }
        }
        // the header, away from the close button, drags the window
        WM_NCHITTEST => {
            let mut p = lparam_point(lp);
            ScreenToClient(hwnd, &mut p);
            let header = with(|app| (p.y as f32) < editor_layout(app.ui).header && app.editor_hit(p).is_none());
            return if header == Some(true) { HTCAPTION as LRESULT } else { HTCLIENT as LRESULT };
        }
        WM_MOUSEMOVE => {
            let p = lparam_point(lp);
            with(|app| {
                let hit = app.editor_hit(p);
                app.editor_hover(hit);
            });
            return 0;
        }
        WM_MOUSELEAVE => {
            with(|app| app.editor_hover(None));
            return 0;
        }
        WM_SETCURSOR => {
            let over = with(|app| app.editor.as_ref().is_some_and(|e| e.draft.hover.is_some()));
            if (lp & 0xFFFF) as u32 == HTCLIENT && over == Some(true) {
                SetCursor(LoadCursorW(null_mut(), IDC_HAND));
                return 1;
            }
        }
        WM_LBUTTONDOWN => {
            let p = lparam_point(lp);
            match with(|app| app.editor_hit(p)).flatten() {
                Some(Hit::Custom) => choose_custom_color(hwnd),
                Some(hit) => {
                    with(|app| app.editor_act(hit));
                }
                None => {}
            }
            return 0;
        }
        WM_COMMAND => {
            let id = (wp & 0xFFFF) as i32;
            let field = (id - ID_EDIT0) as usize;
            match (wp >> 16) as u32 {
                EN_CHANGE => {
                    with(|app| app.update_bubble());
                }
                EN_SETFOCUS => {
                    with(|app| app.editor_focus(Some(field)));
                }
                EN_KILLFOCUS => {
                    with(|app| app.editor_focus(None));
                }
                // Enter and Escape, from IsDialogMessage
                _ if id == IDOK => {
                    with(|app| app.close_editor(true));
                }
                _ if id == IDCANCEL => {
                    with(|app| app.close_editor(false));
                }
                _ => {}
            }
            return 0;
        }
        WM_CLOSE => {
            with(|app| app.close_editor(false));
            return 0;
        }
        _ => {}
    }
    DefWindowProcW(hwnd, msg, wp, lp)
}

/// A text box's words.
pub(super) fn window_text(edit: HWND) -> String {
    let mut buf = [0u16; MAX_CHARS + 1];
    let n = unsafe { GetWindowTextW(edit, buf.as_mut_ptr(), buf.len() as i32) } as usize;
    String::from_utf16_lossy(&buf[..n])
}
