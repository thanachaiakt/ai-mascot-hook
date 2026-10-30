//! The status editor's panel and its text boxes. What it looks like and what its clicks
//! mean is `crate::editor`.

use objc2::runtime::{ProtocolObject, Sel};
use objc2::sel;
use objc2_app_kit::{
    NSColorPanel, NSColorSpace, NSControl, NSControlTextEditingDelegate, NSCursor, NSFocusRingType, NSTextField, NSTextFieldDelegate, NSTextView, NSTrackingArea,
    NSTrackingAreaOptions,
};
use objc2_foundation::{NSNotification, NSObjectProtocol, NSString};

use super::*;

/// The most characters a state's words may have.
const MAX_CHARS: usize = 60;

/// The open status-text editor: its panel, its text boxes and the choices being made.
pub(super) struct Editor {
    win: Retained<NSPanel>,
    /// each state's words, in the order of `STATES`, then `StatusText::extras`
    pub(super) fields: Vec<Retained<NSTextField>>,
    pub(super) draft: Draft,
    canvas: Canvas,
    /// Enter or Escape in a text box: closed on the next tick, once the box is done with
    /// the key (true keeps the changes)
    closing: Option<bool>,
}

define_class!(
    // SAFETY: NSView has no subclassing requirements, and EditorView has no Drop.
    #[unsafe(super(NSView))]
    #[thread_kind = MainThreadOnly]
    #[name = "SealPetEditorView"]
    struct EditorView;

    impl EditorView {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool {
            true
        }

        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, event: &NSEvent) {
            let at = event.locationInWindow();
            match with(|app| app.editor_down(view_pixels(self, at, app.scale))).flatten() {
                // the header, away from the close button, drags the panel
                Some(Down::Drag) => {
                    if let Some(win) = self.window() {
                        win.performWindowDragWithEvent(event);
                    }
                }
                // the color panel calls back as soon as it shows: leave the app free
                Some(Down::Custom) => show_color_panel(self.mtm()),
                None => {}
            }
        }

        #[unsafe(method(mouseMoved:))]
        fn mouse_moved(&self, event: &NSEvent) {
            let at = event.locationInWindow();
            with(|app| {
                let (x, y) = view_pixels(self, at, app.scale);
                let hit = editor_hit(app.scale, x as f32, y as f32);
                app.editor_hover(hit);
            });
        }

        #[unsafe(method(mouseExited:))]
        fn mouse_exited(&self, _event: &NSEvent) {
            with(|app| app.editor_hover(None));
        }
    }
);

impl EditorView {
    fn new(mtm: MainThreadMarker) -> Retained<EditorView> {
        let frame = CGRect::new(CGPoint::new(0.0, 0.0), CGSize::new(1.0, 1.0));
        let view: Retained<EditorView> = unsafe { msg_send![EditorView::alloc(mtm), initWithFrame: frame] };
        let options = NSTrackingAreaOptions::MouseMoved | NSTrackingAreaOptions::MouseEnteredAndExited | NSTrackingAreaOptions::ActiveAlways | NSTrackingAreaOptions::InVisibleRect;
        let area = unsafe { NSTrackingArea::initWithRect_options_owner_userInfo(NSTrackingArea::alloc(), frame, options, Some(&view), None) };
        view.addTrackingArea(&area);
        view
    }
}

/// What a press in the editor leaves for the view to do once the app is free.
enum Down {
    Drag,
    Custom,
}

define_class!(
    // SAFETY: NSObject has no subclassing requirements, and EditorTarget has no Drop.
    #[unsafe(super(NSObject))]
    #[thread_kind = MainThreadOnly]
    #[name = "SealPetEditorTarget"]
    /// The text boxes' delegate, and the color panel's target.
    pub(super) struct EditorTarget;

    impl EditorTarget {
        #[unsafe(method(changeColor:))]
        fn change_color(&self, _sender: Option<&AnyObject>) {
            let color = NSColorPanel::sharedColorPanel(self.mtm()).color();
            if let Some(c) = color.colorUsingColorSpace(&NSColorSpace::sRGBColorSpace()) {
                let byte = |v: f64| (v.clamp(0.0, 1.0) * 255.0).round() as u8;
                let rgba = Rgba(byte(c.redComponent()), byte(c.greenComponent()), byte(c.blueComponent()), 255);
                with(|app| app.pick_color(rgba));
            }
        }
    }

    unsafe impl NSObjectProtocol for EditorTarget {}

    unsafe impl NSControlTextEditingDelegate for EditorTarget {
        #[unsafe(method(controlTextDidChange:))]
        fn control_text_did_change(&self, notification: &NSNotification) {
            if let Some(field) = notification.object().and_then(|o| o.downcast::<NSTextField>().ok()) {
                let text = field.stringValue().to_string();
                if text.chars().count() > MAX_CHARS {
                    field.setStringValue(&NSString::from_str(&text.chars().take(MAX_CHARS).collect::<String>()));
                }
            }
            with(|app| app.update_bubble());
        }

        #[unsafe(method(control:textView:doCommandBySelector:))]
        fn do_command(&self, _control: &NSControl, _text_view: &NSTextView, command: Sel) -> bool {
            let keep = match command {
                c if c == sel!(insertNewline:) => Some(true),
                c if c == sel!(cancelOperation:) => Some(false),
                _ => None,
            };
            if keep.is_some() {
                with(|app| {
                    if let Some(e) = app.editor.as_mut() {
                        e.closing = keep;
                    }
                });
            }
            keep.is_some()
        }
    }

    unsafe impl NSTextFieldDelegate for EditorTarget {}
);

impl EditorTarget {
    pub(super) fn new(mtm: MainThreadMarker) -> Retained<EditorTarget> {
        unsafe { msg_send![EditorTarget::alloc(mtm), init] }
    }
}

fn ns_color(c: Rgba) -> Retained<NSColor> {
    NSColor::colorWithSRGBRed_green_blue_alpha(c.0 as f64 / 255.0, c.1 as f64 / 255.0, c.2 as f64 / 255.0, 1.0)
}

/// Open the system color panel on the selected state's color, outside any borrow of the
/// app: it reports the color it starts on straight away.
fn show_color_panel(mtm: MainThreadMarker) {
    let Some((target, current)) = with(|app| app.editor.as_ref().map(|e| (app.editor_target.clone(), e.draft.colors[e.draft.selected]))).flatten() else { return };
    let panel = NSColorPanel::sharedColorPanel(mtm);
    panel.setShowsAlpha(false);
    panel.setContinuous(true);
    // over the editor, which floats over everything else
    panel.setLevel(NSStatusWindowLevel + 1);
    unsafe {
        panel.setTarget(None);
        panel.setColor(&ns_color(current));
        panel.setTarget(Some(target.as_ref()));
        panel.setAction(Some(sel!(changeColor:)));
    }
    panel.makeKeyAndOrderFront(None);
}

impl App {
    pub(super) fn open_editor(&mut self) {
        if let Some(e) = &self.editor {
            e.win.makeKeyAndOrderFront(None);
            return;
        }
        let ui = self.scale;
        let layout = editor_layout(ui);
        let (ww, wh) = (layout.w, layout.h);
        let s = |v: f32| (v * ui).round() as i32;
        let (mw, _) = self.size();
        let work = self.work_area(self.x + mw / 2, self.y);
        let left_of_seal = self.x - ww - s(12.0);
        let wx = if left_of_seal >= work.0 { left_of_seal } else { (self.x + mw + s(12.0)).min(work.2 - ww) };
        let wy = (self.y - wh / 3).clamp(work.1, (work.3 - wh).max(work.1));

        let view = EditorView::new(self.mtm);
        let win = panel(self.mtm, &view, true, true);
        win.setHasShadow(true);
        if let Some(layer) = view.layer() {
            layer.setCornerRadius(8.0);
            layer.setMasksToBounds(true);
        }

        // each text box sits borderless inside its painted field, centered on one line; the
        // boxes, unlike the canvas, are laid out in points
        let canvas = Canvas::new();
        let line = canvas.measure(self.editor_fonts.edit, "Ag").1 as f64 + 2.0;
        let (pt, inset) = (ui as f64, s(12.0) as f64 / ui as f64);
        let delegate = ProtocolObject::from_ref(&*self.editor_target);
        let draft = Draft::new(&self.status, self.show_status, &self.state);
        let words = self.status.texts.iter().cloned().chain(self.status.extras());
        let fields = layout
            .fields
            .iter()
            .chain(&layout.extra_fields)
            .zip(words)
            .enumerate()
            .map(|(i, (f, words))| {
                let field = NSTextField::textFieldWithString(&NSString::from_str(&words), self.mtm);
                field.setBordered(false);
                field.setBezeled(false);
                field.setDrawsBackground(false);
                field.setFocusRingType(NSFocusRingType::None);
                field.setUsesSingleLineMode(true);
                if let Some(cell) = field.cell() {
                    cell.setScrollable(true);
                }
                field.setFont(Some(unsafe { &*self.editor_fonts.edit }));
                // the boxes past the states' keep the plain ink
                field.setTextColor(Some(&ns_color(draft.colors.get(i).copied().unwrap_or(INK))));
                unsafe { field.setDelegate(Some(delegate)) };
                let top = (f.1 + f.3) as f64 / 2.0 / pt - line / 2.0;
                let frame = CGRect::new(CGPoint::new(f.0 as f64 / pt + inset, wh as f64 / pt - top - line), CGSize::new((f.2 - f.0) as f64 / pt - 2.0 * inset, line));
                field.setFrame(frame);
                view.addSubview(&field);
                field
            })
            .collect::<Vec<_>>();

        win.setAutorecalculatesKeyViewLoop(true);
        let selected = draft.selected;
        self.editor = Some(Editor { win, fields, draft, canvas, closing: None });
        self.paint_editor();
        let e = self.editor.as_ref().unwrap();
        present(self.mtm, &e.win, &e.canvas, wx, wy, 255, self.scale);
        e.win.invalidateShadow();
        e.win.makeKeyAndOrderFront(None);
        e.win.makeFirstResponder(Some(&e.fields[selected]));
        self.sync_clip();
        self.update_bubble();
    }

    fn paint_editor(&mut self) {
        let Some(e) = self.editor.as_mut() else { return };
        e.draft.paint(&mut e.canvas, &self.editor_fonts, self.scale);
        show_canvas(&e.win, &e.canvas, self.scale);
    }

    /// Run on every tick while the editor is open: finish an Enter or Escape, and follow
    /// the keyboard from box to box, which AppKit does not report.
    pub(super) fn editor_tick(&mut self) {
        let Some(e) = &self.editor else { return };
        if let Some(keep) = e.closing {
            return self.close_editor(keep);
        }
        let focus = if e.win.isKeyWindow() { e.fields.iter().position(|f| f.currentEditor().is_some()) } else { None };
        if focus != e.draft.focus {
            self.editor_focus(focus);
        }
    }

    fn editor_focus(&mut self, focus: Option<usize>) {
        let Some(e) = self.editor.as_mut() else { return };
        let moved = e.draft.focus_on(focus);
        self.paint_editor();
        if moved {
            self.sync_clip();
            self.update_bubble();
        }
    }

    /// Note what the pointer is over, repainting when that changes.
    fn editor_hover(&mut self, hit: Option<Hit>) {
        let Some(e) = self.editor.as_mut() else { return };
        // the text boxes show their own cursor
        let cursor = if hit.is_some() { NSCursor::pointingHandCursor() } else { NSCursor::arrowCursor() };
        cursor.set();
        if e.draft.hover != hit {
            e.draft.hover = hit;
            self.paint_editor();
        }
    }

    /// A press at `at`, in the editor's pixels. Does what it means, or leaves the view
    /// something to do with the app free.
    fn editor_down(&mut self, at: (i32, i32)) -> Option<Down> {
        let (x, y) = (at.0 as f32, at.1 as f32);
        let Some(hit) = editor_hit(self.scale, x, y) else {
            return (y < editor_layout(self.scale).header).then_some(Down::Drag);
        };
        match hit {
            Hit::Close | Hit::Cancel => self.close_editor(false),
            Hit::Save => self.close_editor(true),
            Hit::Reset => self.reset_editor(),
            Hit::Swatch(i) => self.pick_color(SWATCHES[i]),
            Hit::Custom => return Some(Down::Custom),
            Hit::State(i) => {
                let e = self.editor.as_ref()?;
                e.win.makeFirstResponder(Some(&e.fields[i]));
            }
            _ => {
                self.editor.as_mut()?.draft.act(hit);
                self.paint_editor();
            }
        }
        None
    }

    /// Color the selected state's words.
    fn pick_color(&mut self, c: Rgba) {
        let Some(e) = self.editor.as_mut() else { return };
        e.draft.colors[e.draft.selected] = c;
        e.fields[e.draft.selected].setTextColor(Some(&ns_color(c)));
        self.paint_editor();
        self.update_bubble();
    }

    fn reset_editor(&mut self) {
        let defaults = StatusText::defaults();
        let Some(e) = self.editor.as_mut() else { return };
        let words = defaults.texts.iter().cloned().chain(defaults.extras());
        for (field, text) in e.fields.iter().zip(words) {
            let text = NSString::from_str(&text);
            field.setStringValue(&text);
            // a box being typed in shows its field editor's copy of the words
            if let Some(editing) = field.currentEditor() {
                editing.setString(&text);
            }
        }
        e.draft.colors = defaults.colors;
        for (field, c) in e.fields.iter().zip(defaults.colors) {
            field.setTextColor(Some(&ns_color(c)));
        }
        self.paint_editor();
        self.update_bubble();
    }

    pub(super) fn close_editor(&mut self, keep: bool) {
        let Some(e) = self.editor.take() else { return };
        if keep {
            let extras = std::array::from_fn(|i| e.fields[EXTRAS + i].stringValue().to_string());
            e.draft.commit(&mut self.status, std::array::from_fn(|i| e.fields[i].stringValue().to_string()), extras);
            self.show_status = e.draft.show;
            let _ = std::fs::create_dir_all(data_dir());
            self.status.save(&data_dir().join("status.txt"));
            self.save();
        }
        if NSColorPanel::sharedColorPanelExists(self.mtm) {
            let panel = NSColorPanel::sharedColorPanel(self.mtm);
            unsafe { panel.setTarget(None) };
            panel.orderOut(None);
        }
        e.win.orderOut(None);
        NSCursor::arrowCursor().set();
        self.sync_clip();
        self.restart_autohide();
    }
}
