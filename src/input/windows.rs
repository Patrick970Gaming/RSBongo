use super::AppEvent;
use crossbeam_channel::Sender;
use std::cell::RefCell;
use std::ptr;
use std::thread;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DispatchMessageW, GetMessageW, SetWindowsHookExW, TranslateMessage,
    UnhookWindowsHookEx, HC_ACTION, MSG, WH_KEYBOARD_LL, WH_MOUSE_LL, WM_KEYDOWN, WM_KEYUP,
    WM_LBUTTONDOWN, WM_LBUTTONUP, WM_MBUTTONDOWN, WM_MBUTTONUP, WM_RBUTTONDOWN, WM_RBUTTONUP,
    WM_SYSKEYDOWN, WM_SYSKEYUP,
};

thread_local! {
    // Low-level hook callbacks are plain `extern "system" fn` pointers —
    // no closures allowed — so the channel sender has to live somewhere
    // the callback can reach without being captured. Thread-local is
    // safe here because WH_KEYBOARD_LL / WH_MOUSE_LL hooks only ever
    // fire on the thread that installed them (this thread's message
    // pump, below), never from another thread.
    static SENDER: RefCell<Option<Sender<AppEvent>>> = RefCell::new(None);
}

unsafe extern "system" fn keyboard_hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code == HC_ACTION as i32 {
        let event = match wparam as u32 {
            WM_KEYDOWN | WM_SYSKEYDOWN => Some(AppEvent::KeyPressed),
            WM_KEYUP | WM_SYSKEYUP => Some(AppEvent::KeyReleased),
            _ => None,
        };
        if let Some(ev) = event {
            SENDER.with(|s| {
                if let Some(sender) = s.borrow().as_ref() {
                    let _ = sender.send(ev);
                }
            });
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

unsafe extern "system" fn mouse_hook_proc(code: i32, wparam: usize, lparam: isize) -> isize {
    if code == HC_ACTION as i32 {
        let event = match wparam as u32 {
            WM_LBUTTONDOWN | WM_RBUTTONDOWN | WM_MBUTTONDOWN => Some(AppEvent::KeyPressed),
            WM_LBUTTONUP | WM_RBUTTONUP | WM_MBUTTONUP => Some(AppEvent::KeyReleased),
            _ => None,
        };
        if let Some(ev) = event {
            SENDER.with(|s| {
                if let Some(sender) = s.borrow().as_ref() {
                    let _ = sender.send(ev);
                }
            });
        }
    }
    CallNextHookEx(ptr::null_mut(), code, wparam, lparam)
}

/// Installs system-wide low-level keyboard + mouse hooks on a
/// dedicated thread and runs the Windows message pump they require to
/// keep firing. This is the Windows equivalent of the Linux evdev
/// backend: captures input regardless of which window has focus.
pub fn spawn_listeners(sender: Sender<AppEvent>) {
    thread::spawn(move || unsafe {
        SENDER.with(|s| *s.borrow_mut() = Some(sender));

        let kb_hook = SetWindowsHookExW(WH_KEYBOARD_LL, Some(keyboard_hook_proc), ptr::null_mut(), 0);
        let mouse_hook = SetWindowsHookExW(WH_MOUSE_LL, Some(mouse_hook_proc), ptr::null_mut(), 0);

        if kb_hook.is_null() {
            eprintln!("[windows] failed to install keyboard hook");
        }
        if mouse_hook.is_null() {
            eprintln!("[windows] failed to install mouse hook");
        }

        // WH_KEYBOARD_LL / WH_MOUSE_LL only fire while this thread is
        // actively pumping messages — this blocks here for the life
        // of the app, which is fine since it's on its own thread.
        let mut msg: MSG = std::mem::zeroed();
        while GetMessageW(&mut msg, ptr::null_mut(), 0, 0) > 0 {
            TranslateMessage(&msg);
            DispatchMessageW(&msg);
        }

        if !kb_hook.is_null() {
            UnhookWindowsHookEx(kb_hook);
        }
        if !mouse_hook.is_null() {
            UnhookWindowsHookEx(mouse_hook);
        }
    });
}
