//! Main-thread floating strips; the worker exchanges only portable values.
use crate::api::{
    Screen,
    window::WindowId,
    window_tabs::{TabBar, TabDrop, TabGroupId, TabNativeEvent, WindowTarget},
};
use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, MainThreadOnly, define_class, msg_send};
use objc2_app_kit::{
    NSBackingStoreType, NSButton, NSButtonType, NSEvent, NSPanel, NSTextField, NSView,
    NSWindowOrderingMode, NSWindowStyleMask,
};
use objc2_foundation::{NSPoint, NSRect, NSSize, NSString};
use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::sync::Mutex;

#[derive(Default)]
struct PendingBars {
    full: Option<Vec<(TabBar, Option<isize>)>>,
    updates: BTreeMap<u32, (TabBar, Option<isize>)>,
}
static PENDING: Mutex<PendingBars> = Mutex::new(PendingBars {
    full: None,
    updates: BTreeMap::new(),
});
static EVENTS: Mutex<Vec<TabNativeEvent>> = Mutex::new(Vec::new());
thread_local! { static STRIPS: RefCell<BTreeMap<u32, Strip>> = const { RefCell::new(BTreeMap::new()) }; }

struct ButtonState {
    group: TabGroupId,
    event: Option<TabNativeEvent>,
    source: Option<WindowTarget>,
    down: Cell<Option<NSPoint>>,
    dragging: Cell<bool>,
}
define_class!(
    #[unsafe(super(NSButton))]
    #[thread_kind = MainThreadOnly]
    #[name = "KeySteerTabButton"]
    #[ivars = ButtonState]
    struct TabButton;
    impl TabButton {
        #[unsafe(method(acceptsFirstMouse:))]
        fn accepts_first_mouse(&self, _event: Option<&NSEvent>) -> bool { true }
        #[unsafe(method(mouseDown:))]
        fn mouse_down(&self, _event: &NSEvent) {
            self.ivars().down.set(Some(NSEvent::mouseLocation()));
            self.ivars().dragging.set(false);
        }
        #[unsafe(method(mouseDragged:))]
        fn mouse_dragged(&self, _event: &NSEvent) {
            let point = NSEvent::mouseLocation();
            if self.ivars().source.is_some() && self.ivars().down.get().is_some_and(|start|
                (point.x - start.x).hypot(point.y - start.y) >= 5.0) {
                self.ivars().dragging.set(true);
                show_drop(point);
            }
        }
        #[unsafe(method(mouseUp:))]
        fn mouse_up(&self, _event: &NSEvent) {
            let point = NSEvent::mouseLocation();
            if self.ivars().down.replace(None).is_none() { return; }
            if self.ivars().dragging.replace(false) {
                if let Some(source) = self.ivars().source
                    && let Some((target, before)) = drop_at(point) {
                    enqueue(TabNativeEvent::Drop(TabDrop { source, target, before }));
                }
            } else if self.window().is_some_and(|window| {
                let local = self.convertPoint_fromView(window.convertPointFromScreen(point), None);
                local.x >= 0.0 && local.y >= 0.0 && local.x < self.frame().size.width && local.y < 30.0
            }) && let Some(event) = self.ivars().event {
                enqueue(event);
            }
            clear_drop();
        }
        #[unsafe(method(scrollWheel:))]
        fn scroll_wheel(&self, event: &NSEvent) {
            let delta = if event.scrollingDeltaX().abs() > event.scrollingDeltaY().abs() {
                event.scrollingDeltaX()
            } else { event.scrollingDeltaY() };
            STRIPS.with(|strips| {
                if let Some(strip) = strips.borrow_mut().get_mut(&self.ivars().group.0) {
                    strip.offset -= delta * if event.hasPreciseScrollingDeltas() { 1.0 } else { 20.0 };
                    strip.layout(false);
                }
            });
        }
        #[unsafe(method(cancelOperation:))]
        fn cancel_operation(&self, _sender: Option<&objc2::runtime::NSObject>) {
            self.ivars().down.set(None);
            self.ivars().dragging.set(false);
            clear_drop();
        }
    }
);
fn button(
    mtm: MainThreadMarker,
    group: TabGroupId,
    title: &str,
    event: Option<TabNativeEvent>,
    source: Option<WindowTarget>,
) -> Retained<TabButton> {
    let allocated = TabButton::alloc(mtm).set_ivars(ButtonState {
        group,
        event,
        source,
        down: Cell::new(None),
        dragging: Cell::new(false),
    });
    // SAFETY: initialize the NSButton subclass on the AppKit thread; its Rust
    // ivars are installed before init and retained by the containing strip.
    let button: Retained<TabButton> = unsafe { msg_send![super(allocated), init] };
    button.setButtonType(NSButtonType::PushOnPushOff);
    button.setTitle(&NSString::from_str(title));
    button.setToolTip(Some(&NSString::from_str(title)));
    button.setKeyEquivalent(&NSString::from_str(""));
    button
}
struct Strip {
    panel: Retained<NSPanel>,
    viewport: Retained<NSView>,
    heading: Retained<TabButton>,
    close: Retained<TabButton>,
    insertion: Retained<NSTextField>,
    buttons: Vec<Retained<TabButton>>,
    tabs: Vec<(WindowId, u32, String)>,
    layout_width: f64,
    offset: f64,
    active: Option<WindowId>,
    owner: Option<isize>,
}
fn tab_width(width: f64, count: usize) -> f64 {
    ((width - 74.0).max(1.0) / count.max(1) as f64).max(100.0)
}
fn insertion_index(x: f64, width: f64, count: usize) -> usize {
    ((x / width + 0.5).floor().max(0.0) as usize).min(count)
}
fn scroll_offset(
    offset: f64,
    viewport: f64,
    width: f64,
    count: usize,
    selected: Option<usize>,
) -> f64 {
    let mut offset = offset;
    if let Some(index) = selected {
        let left = index as f64 * width;
        if left < offset {
            offset = left;
        } else if left + width > offset + viewport {
            offset = left + width - viewport;
        }
    }
    offset.clamp(0.0, (width * count as f64 - viewport).max(0.0))
}
impl Strip {
    fn layout(&mut self, reveal: bool) {
        let viewport = (self.layout_width - 74.0).max(1.0);
        let width = tab_width(self.layout_width, self.tabs.len());
        self.offset = scroll_offset(
            self.offset,
            viewport,
            width,
            self.tabs.len(),
            reveal
                .then(|| {
                    self.tabs
                        .iter()
                        .position(|(id, _, _)| Some(*id) == self.active)
                })
                .flatten(),
        );
        self.viewport.setFrame(NSRect::new(
            NSPoint::new(44.0, 0.0),
            NSSize::new(viewport, 30.0),
        ));
        self.heading
            .setFrame(NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(44.0, 30.0)));
        self.close.setFrame(NSRect::new(
            NSPoint::new(self.layout_width - 30.0, 0.0),
            NSSize::new(30.0, 30.0),
        ));
        for (i, button) in self.buttons.iter().enumerate() {
            button.setFrame(NSRect::new(
                NSPoint::new(i as f64 * width - self.offset, 0.0),
                NSSize::new(width, 30.0),
            ));
            button.setState(if Some(self.tabs[i].0) == self.active {
                1
            } else {
                0
            });
        }
    }
    fn drop_position(&self, point: NSPoint) -> Option<(Option<WindowId>, f64)> {
        let frame = self.panel.frame();
        if !self.panel.isVisible()
            || point.x < frame.origin.x
            || point.x >= frame.origin.x + frame.size.width
            || point.y < frame.origin.y
            || point.y >= frame.origin.y + frame.size.height
        {
            return None;
        }
        let width = tab_width(self.layout_width, self.tabs.len());
        let x = point.x - frame.origin.x - 44.0 + self.offset;
        let index = insertion_index(x, width, self.tabs.len());
        Some((
            self.tabs.get(index).map(|(id, _, _)| *id),
            (44.0 + index as f64 * width - self.offset)
                .clamp(44.0, (self.layout_width - 30.0).max(44.0)),
        ))
    }
}
fn drop_at(point: NSPoint) -> Option<(TabGroupId, Option<WindowId>)> {
    STRIPS.with(|strips| {
        strips.borrow().iter().find_map(|(id, strip)| {
            strip
                .drop_position(point)
                .map(|(before, _)| (TabGroupId(*id), before))
        })
    })
}
fn show_drop(point: NSPoint) {
    STRIPS.with(|strips| {
        for strip in strips.borrow().values() {
            let position = strip.drop_position(point);
            strip.insertion.setHidden(position.is_none());
            if let Some((_, x)) = position {
                strip.insertion.setFrame(NSRect::new(
                    NSPoint::new(x - 2.0, 2.0),
                    NSSize::new(4.0, 26.0),
                ));
            }
        }
    });
}
fn clear_drop() {
    STRIPS.with(|strips| {
        for strip in strips.borrow().values() {
            strip.insertion.setHidden(true);
        }
    });
}
impl Drop for Strip {
    fn drop(&mut self) {
        self.panel.orderOut(None);
    }
}
type WorkerWaker = std::sync::Arc<dyn Fn() + Send + Sync>;
static WORKER_WAKER: Mutex<Option<WorkerWaker>> = Mutex::new(None);
static MOUSE_RELEASED: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
pub(super) fn set_worker_waker(wake: Option<WorkerWaker>) {
    *WORKER_WAKER.lock().unwrap_or_else(|e| e.into_inner()) = wake;
}
fn wake_worker() {
    if let Some(wake) = WORKER_WAKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        wake();
    }
}
pub(super) fn mouse_released() {
    MOUSE_RELEASED.store(true, std::sync::atomic::Ordering::Release);
    wake_worker();
}
pub(super) fn take_mouse_release() -> bool {
    MOUSE_RELEASED.swap(false, std::sync::atomic::Ordering::AcqRel)
}
pub(super) fn enqueue(event: TabNativeEvent) {
    enqueue_native(event);
    wake_worker();
}
// AX callbacks are already executing on the awakened window run loop.
// Queue their identity without sending a redundant pipe wake back to it.
pub(super) fn enqueue_native(event: TabNativeEvent) {
    let mut events = EVENTS.lock().unwrap_or_else(|e| e.into_inner());
    if !events.contains(&event) && events.len() < 512 {
        events.push(event);
    }
}
static MAIN_WAKER: Mutex<Option<WorkerWaker>> = Mutex::new(None);
thread_local! { static MAIN_WAKE: std::cell::OnceCell<super::native::RunLoopWake> = const { std::cell::OnceCell::new() }; }
fn wake_main() {
    if let Some(wake) = MAIN_WAKER
        .lock()
        .unwrap_or_else(|e| e.into_inner())
        .as_ref()
    {
        wake();
    } else {
        super::workspace::wake_main_run_loop();
    }
}
pub(super) fn publish(bars: &[TabBar], owners: &[(WindowId, isize)]) {
    if let Ok(mut pending) = PENDING.lock() {
        pending.full = Some(
            bars.iter()
                .map(|bar| {
                    (
                        bar.clone(),
                        owners
                            .iter()
                            .find(|(id, _)| *id == bar.active)
                            .map(|(_, number)| *number),
                    )
                })
                .collect(),
        );
        pending.updates.clear();
        wake_main();
    }
}
pub(super) fn publish_one(bar: &TabBar, owner: Option<isize>) -> Result<(), String> {
    let mut pending = PENDING
        .lock()
        .map_err(|_| "Tab publication queue poisoned")?;
    if let Some(full) = &mut pending.full {
        if let Some(current) = full
            .iter_mut()
            .find(|(current, _)| current.group == bar.group)
        {
            *current = (bar.clone(), owner);
        }
    } else {
        pending.updates.insert(bar.group.0, (bar.clone(), owner));
    }
    drop(pending);
    wake_main();
    Ok(())
}
pub(super) fn take_events() -> Vec<TabNativeEvent> {
    EVENTS
        .lock()
        .map(|mut events| std::mem::take(&mut *events))
        .unwrap_or_default()
}
pub(super) fn clear(mtm: MainThreadMarker) {
    refresh(mtm, &[]);
    STRIPS.with(|strips| strips.borrow_mut().clear());
}
pub(super) fn refresh(mtm: MainThreadMarker, screens: &[Screen]) {
    MAIN_WAKE.with(|slot| {
        if slot.get().is_none()
            && let Some(wake) = super::native::RunLoopWake::new()
        {
            *MAIN_WAKER.lock().unwrap_or_else(|e| e.into_inner()) = Some(wake.waker());
            let _ = slot.set(wake);
        }
        if let Some(wake) = slot.get() {
            wake.drain();
        }
    });
    thread_local! { static FOREGROUND: std::cell::Cell<i32> = const { std::cell::Cell::new(0) }; }
    let pid = objc2_app_kit::NSWorkspace::sharedWorkspace()
        .frontmostApplication()
        .map_or(0, |app| app.processIdentifier());
    FOREGROUND.with(|previous| {
        if previous.replace(pid) != pid {
            enqueue(TabNativeEvent::VisibilityChanged);
        }
    });
    let Some(pending) = PENDING.lock().ok().map(|mut p| std::mem::take(&mut *p)) else {
        return;
    };
    let structural = pending.full.is_some();
    let bars = pending
        .full
        .unwrap_or_else(|| pending.updates.into_values().collect());
    if bars.is_empty() && !structural {
        return;
    }
    let top = screens
        .iter()
        .find(|s| s.is_primary)
        .map_or(0.0, |s| s.bounds.bottom());
    STRIPS.with(|strips| {
        let mut strips = strips.borrow_mut();
        if structural {
            strips.retain(|id, _| bars.iter().any(|(bar, _)| bar.group.0 == *id));
        }
        for (bar, owner) in bars {
            if !structural && !strips.contains_key(&bar.group.0) {
                continue;
            }
            let strip = strips.entry(bar.group.0).or_insert_with(|| {
                let panel = NSPanel::initWithContentRect_styleMask_backing_defer(
                    NSPanel::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 30.0)),
                    NSWindowStyleMask::Borderless | NSWindowStyleMask::NonactivatingPanel,
                    NSBackingStoreType::Buffered,
                    false,
                );
                panel.setHidesOnDeactivate(false);
                panel.setBecomesKeyOnlyIfNeeded(true);
                panel.setHasShadow(false);
                panel.setLevel(0);
                let content = NSView::initWithFrame(
                    NSView::alloc(mtm),
                    NSRect::new(NSPoint::new(0.0, 0.0), NSSize::new(1.0, 30.0)),
                );
                let viewport = NSView::initWithFrame(NSView::alloc(mtm), content.frame());
                viewport.setClipsToBounds(true);
                let heading = button(
                    mtm,
                    bar.group,
                    &format!("~{}", bar.group.0),
                    None,
                    Some(WindowTarget::Group(bar.group)),
                );
                let close = button(
                    mtm,
                    bar.group,
                    "×",
                    Some(TabNativeEvent::Dissolve(bar.group)),
                    None,
                );
                let insertion = NSTextField::labelWithString(&NSString::from_str("│"), mtm);
                insertion.setHidden(true);
                content.addSubview(&viewport);
                content.addSubview(&heading);
                content.addSubview(&close);
                content.addSubview(&insertion);
                panel.setContentView(Some(&content));
                Strip {
                    panel,
                    viewport,
                    heading,
                    close,
                    insertion,
                    buttons: Vec::new(),
                    tabs: Vec::new(),
                    layout_width: 0.0,
                    offset: 0.0,
                    active: None,
                    owner: None,
                }
            });
            let layout_changed = strip.layout_width != bar.bounds.width || strip.tabs != bar.tabs;
            let selection_changed = strip.active != Some(bar.active);
            if strip.tabs != bar.tabs {
                // Keep existing controls when only titles/numbers change.
                if strip
                    .tabs
                    .iter()
                    .map(|t| t.0)
                    .ne(bar.tabs.iter().map(|t| t.0))
                {
                    for button in &strip.buttons {
                        button.removeFromSuperview();
                    }
                    strip.buttons.clear();
                    for (id, _, _) in &bar.tabs {
                        let button = button(
                            mtm,
                            bar.group,
                            "",
                            Some(TabNativeEvent::Activate(*id)),
                            Some(WindowTarget::Window(*id)),
                        );
                        strip.viewport.addSubview(&button);
                        strip.buttons.push(button);
                    }
                }
                for (button, (_, n, title)) in strip.buttons.iter().zip(&bar.tabs) {
                    let title = if *n == 0 {
                        title.clone()
                    } else {
                        format!("{n}  {title}")
                    };
                    button.setTitle(&NSString::from_str(&title));
                    button.setToolTip(Some(&NSString::from_str(&title)));
                }
                strip.tabs.clone_from(&bar.tabs);
            }
            strip.layout_width = bar.bounds.width;
            strip.active = Some(bar.active);
            if layout_changed || selection_changed {
                strip.layout(selection_changed);
            }
            let above = bar.bounds.y - 30.0;
            let y = above;
            strip.panel.setFrame_display(
                NSRect::new(
                    NSPoint::new(bar.bounds.x, top - y - 30.0),
                    NSSize::new(bar.bounds.width, 30.0),
                ),
                layout_changed || selection_changed,
            );
            if bar.visible
                && screens
                    .get(bar.screen)
                    .is_none_or(|screen| above >= screen.work_area.y - 1.0)
            {
                if structural || !strip.panel.isVisible() || strip.owner != owner {
                    if let Some(owner) = owner {
                        // Keep the strip immediately above its application window,
                        // below unrelated foreground applications at the normal level.
                        strip
                            .panel
                            .orderWindow_relativeTo(NSWindowOrderingMode::Above, owner);
                    } else {
                        strip.panel.orderFront(None);
                    }
                }
            } else if strip.panel.isVisible() {
                strip.panel.orderOut(None);
            }
            strip.owner = owner;
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn drop_insertion_uses_the_same_scrolled_coordinates_as_drawing() {
        let width = tab_width(474.0, 10);
        let offset = 250.0;
        // First visible fragment belongs to member 2; its right half inserts
        // before member 3. Far left/right drops stay within the group.
        assert_eq!(insertion_index(offset, width, 10), 3);
        assert_eq!(insertion_index(offset + 101.0, width, 10), 4);
        assert_eq!(insertion_index(-44.0, width, 10), 0);
        assert_eq!(insertion_index(2000.0, width, 10), 10);
    }
    #[test]
    fn scrolling_preserves_readable_tabs_and_reveals_selection() {
        let width = tab_width(474.0, 10);
        assert_eq!(width, 100.0);
        assert_eq!(scroll_offset(0.0, 400.0, width, 10, Some(9)), 600.0);
        assert_eq!(scroll_offset(600.0, 400.0, width, 10, Some(0)), 0.0);
        assert_eq!(scroll_offset(250.0, 400.0, width, 10, None), 250.0);
        assert_eq!(scroll_offset(600.0, 400.0, width, 3, None), 0.0);
        assert_eq!(scroll_offset(-10.0, 400.0, width, 10, None), 0.0);
    }
}
