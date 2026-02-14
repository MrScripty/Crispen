//! Offscreen rendering capture (BGRA format).
//!
//! Utilities for extracting the CEF framebuffer.  The `Arc<Vec<u8>>`
//! wrapper enables zero-copy sharing (~20 ns clone vs ~6–12 ms memcpy
//! for an 18 MB HiDPI buffer).

use crate::browser::SharedState;
use crispen_frontend_core::CaptureResult;
use std::sync::atomic::Ordering;
use std::sync::Arc;

/// Return the current framebuffer if it has changed since the last call.
pub(crate) fn capture_if_dirty(shared: &Arc<SharedState>) -> Option<CaptureResult> {
    if !shared.dirty.swap(false, Ordering::SeqCst) {
        return None;
    }

    let buffer = shared.framebuffer.lock().unwrap().clone()?;
    let (width, height) = *shared.framebuffer_size.lock().unwrap();

    Some(CaptureResult::Bgra(buffer, width, height))
}

/// Whether a framebuffer has been received at all.
pub(crate) fn has_framebuffer(shared: &Arc<SharedState>) -> bool {
    shared.framebuffer.lock().unwrap().is_some()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicBool;
    use std::sync::Mutex;
    use tokio::sync::mpsc;

    fn test_shared() -> Arc<SharedState> {
        let (tx, _rx) = mpsc::unbounded_channel();
        Arc::new(SharedState {
            framebuffer: Mutex::new(None),
            framebuffer_size: Mutex::new((0, 0)),
            dirty: Arc::new(AtomicBool::new(false)),
            size: Mutex::new((800, 600)),
            from_ui_tx: tx,
        })
    }

    #[test]
    fn not_dirty_returns_none() {
        assert!(capture_if_dirty(&test_shared()).is_none());
    }

    #[test]
    fn dirty_without_buffer_returns_none() {
        let s = test_shared();
        s.dirty.store(true, Ordering::SeqCst);
        assert!(capture_if_dirty(&s).is_none());
    }

    #[test]
    fn dirty_with_buffer_returns_bgra() {
        let s = test_shared();
        *s.framebuffer.lock().unwrap() = Some(Arc::new(vec![0u8; 800 * 600 * 4]));
        *s.framebuffer_size.lock().unwrap() = (800, 600);
        s.dirty.store(true, Ordering::SeqCst);

        match capture_if_dirty(&s) {
            Some(CaptureResult::Bgra(buf, w, h)) => {
                assert_eq!((w, h), (800, 600));
                assert_eq!(buf.len(), 800 * 600 * 4);
            }
            other => panic!("expected Bgra, got {other:?}"),
        }

        assert!(!s.dirty.load(Ordering::SeqCst));
    }
}
