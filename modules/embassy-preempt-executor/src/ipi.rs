//! IPI (Inter-Processor Interrupt) wait mechanism
//!
//! Provides an async interface for waiting on IPI events from other harts.
//!
//! ## Architecture
//!
//! ```text
//! wait_for_ipi().await
//!       │
//!       ├─→ first poll: register task in IPI wait list
//!       │       └─→ single_poll: remove from ready queue
//!       │
//!       └─→ MSIP interrupt (from another hart)
//!               └─→ ipi_callback()
//!                       ├─→ wake all tasks in wait list
//!                       └─→ IntCtxSW() → context switch
//!                               └─→ task resumes: poll returns Ready
//! ```

use core::future::Future;
use core::pin::Pin;
use core::task::{Context, Poll};

use embassy_preempt_structs::cell::SyncUnsafeCell;

use crate::{wake_task_no_pend, GlobalSyncExecutor, OS_TCB_REF};

// ============================================================================
// IPI Wait List
// ============================================================================

/// Maximum number of tasks that can wait for IPI simultaneously
const IPI_WAIT_LIST_SIZE: usize = 4;

/// A slot in the IPI wait list
struct IpiWaitSlot {
    task: SyncUnsafeCell<Option<OS_TCB_REF>>,
}

/// Global IPI wait list
static IPI_WAIT_LIST: IpiWaitList = IpiWaitList::new();

struct IpiWaitList {
    slots: [IpiWaitSlot; IPI_WAIT_LIST_SIZE],
}

impl IpiWaitList {
    const fn new() -> Self {
        Self {
            slots: [const { IpiWaitSlot {
                task: SyncUnsafeCell::new(None),
            } }; IPI_WAIT_LIST_SIZE],
        }
    }

    /// Add a task to the wait list. Returns false if full.
    fn add(&self, task: OS_TCB_REF) -> bool {
        for slot in &self.slots {
            if slot.task.get_unmut().is_none() {
                unsafe { slot.task.set(Some(task)); }
                return true;
            }
        }
        false
    }

    /// Remove a task from the wait list.
    fn remove(&self, task: OS_TCB_REF) {
        for slot in &self.slots {
            let current = slot.task.get_unmut();
            if let Some(t) = *current {
                if t.as_ptr() == task.as_ptr() {
                    unsafe { slot.task.set(None); }
                    return;
                }
            }
        }
    }

    /// Wake all waiting tasks (called from IPI interrupt callback).
    fn wake_all(&self) {
        for slot in &self.slots {
            let current = slot.task.get_unmut();
            if let Some(task) = *current {
                unsafe { slot.task.set(None); }
                wake_task_no_pend(task);
            }
        }
    }
}

// ============================================================================
// IPI Callback (called from platform MSIP handler)
// ============================================================================

/// IPI interrupt callback. Registered with the platform during executor init.
///
/// Called from the MSIP interrupt handler. Wakes all waiting tasks and
/// triggers a context switch.
pub(crate) fn ipi_callback(_ctx: *mut ()) {
    scheduler_log!(trace, "IPI callback: waking waiting tasks");
    IPI_WAIT_LIST.wake_all();
    unsafe {
        GlobalSyncExecutor().as_ref().unwrap().IntCtxSW();
    }
}

/// Register the IPI callback with the platform. Called during executor init.
pub(crate) fn init() {
    embassy_preempt_platform::get_platform_trait()
        .set_ipi_callback(ipi_callback, core::ptr::null_mut());
    scheduler_log!(info, "IPI callback registered with platform");
}

// ============================================================================
// IPI Wait Future
// ============================================================================

/// A future that completes when an IPI is received.
///
/// # Usage
///
/// ```ignore
/// async fn my_task(_args: *mut c_void) {
///     loop {
///         wait_for_ipi().await;
///         task_log!(info, "Got IPI!");
///     }
/// }
/// ```
#[must_use = "futures do nothing unless you `.await` or poll them"]
pub struct IpiWait {
    registered: bool,
}

impl Unpin for IpiWait {}

impl Future for IpiWait {
    type Output = ();

    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.registered {
            // Task was registered and has been woken by IPI — complete
            self.registered = false;
            scheduler_log!(trace, "IpiWait: completed");
            Poll::Ready(())
        } else {
            // Register in the wait list; single_poll will remove from ready queue
            let task = crate::waker::task_from_waker(cx.waker());
            if IPI_WAIT_LIST.add(task) {
                self.registered = true;
                scheduler_log!(trace, "IpiWait: task registered, waiting");
                Poll::Pending
            } else {
                scheduler_log!(error, "IpiWait: wait list full!");
                Poll::Ready(())
            }
        }
    }
}

impl Drop for IpiWait {
    fn drop(&mut self) {
        if self.registered {
            // Future dropped while still waiting — clean up from wait list
            // The task ref is not easily accessible here, but wake_all handles cleanup.
            // For safety, the next wake_all will simply skip None slots.
        }
    }
}

/// Create a future that waits for the next IPI (Inter-Processor Interrupt).
///
/// When `.await`ed, the current async task is suspended until an IPI arrives.
///
/// # Example
///
/// ```ignore
/// async fn ipi_handler_task(_args: *mut c_void) {
///     loop {
///         wait_for_ipi().await;
///         task_log!(info, "Received IPI from another hart!");
///     }
/// }
/// ```
pub fn wait_for_ipi() -> IpiWait {
    IpiWait { registered: false }
}
