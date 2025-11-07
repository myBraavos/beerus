use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use tokio::sync::Notify;

#[derive(Clone)]
pub struct AsyncBlocker {
    task_count: Arc<AtomicUsize>,
    notify: Arc<Notify>,
}

impl AsyncBlocker {
    pub fn new() -> Self {
        Self {
            task_count: Arc::new(AtomicUsize::new(0)),
            notify: Arc::new(Notify::new()),
        }
    }

    /// Increments the internal task counter to indicate a "lock" is held.
    /// When this function is called, background loader tasks (that call `wait_for_unlock`)
    /// will pause until all such "locks" have been released (when their AsyncTaskGuard is dropped).
    ///
    /// Returns an `AsyncTaskGuard` which will automatically decrement the counter and potentially
    /// notify waiting tasks when dropped.
    ///
    /// Usage:
    /// ```
    /// let _guard = async_blocker.block_tasks();
    /// // do something
    /// ```
    pub fn block_tasks(&self) -> AsyncTaskGuard {
        self.task_count.fetch_add(1, Ordering::SeqCst);
        AsyncTaskGuard { b: self.clone() }
    }

    pub async fn wait_for_unlock(&self) {
        loop {
            if self.task_count.load(Ordering::SeqCst) == 0 {
                tracing::debug!("No tasks to block, unlocking");
                return;
            }
            self.notify.notified().await;
        }
    }
}

impl Default for AsyncBlocker {
    fn default() -> Self {
        Self::new()
    }
}

pub struct AsyncTaskGuard {
    b: AsyncBlocker,
}

impl Drop for AsyncTaskGuard {
    fn drop(&mut self) {
        if self.b.task_count.fetch_sub(1, Ordering::SeqCst) == 1 {
            self.b.notify.notify_waiters();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::time::timeout;

    #[test]
    fn test_new() {
        let blocker = AsyncBlocker::new();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_default() {
        let blocker = AsyncBlocker::default();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn test_block_tasks_increments_counter() {
        let blocker = AsyncBlocker::new();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);

        let _guard = blocker.block_tasks();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 1);

        let _guard2 = blocker.block_tasks();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 2);
    }

    #[test]
    fn test_guard_drop_decrements_counter() {
        let blocker = AsyncBlocker::new();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);

        let guard = blocker.block_tasks();
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 1);

        drop(guard);
        assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);
    }

    #[tokio::test]
    async fn test_wait_for_unlock_no_blocks() {
        let blocker = AsyncBlocker::new();
        // Should return immediately when no blocks are active
        blocker.wait_for_unlock().await;
    }

    #[tokio::test]
    async fn test_wait_for_unlock_with_block() {
        let blocker = AsyncBlocker::new();
        let guard = blocker.block_tasks();

        // Spawn a task that waits for unlock
        let blocker_clone = blocker.clone();
        let wait_handle = tokio::spawn(async move {
            blocker_clone.wait_for_unlock().await;
        });

        // Give it a moment to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // The wait should still be pending
        assert!(!wait_handle.is_finished());

        // Drop the guard, which should notify waiters
        drop(guard);

        // Now wait_for_unlock should complete
        let result = timeout(Duration::from_millis(100), wait_handle).await;
        assert!(
            result.is_ok(),
            "wait_for_unlock should complete after guard is dropped"
        );
    }

    #[tokio::test]
    async fn test_wait_for_unlock_multiple_guards() {
        let blocker = AsyncBlocker::new();
        let guard1 = blocker.block_tasks();
        let guard2 = blocker.block_tasks();

        // Spawn a task that waits for unlock
        let blocker_clone = blocker.clone();
        let wait_handle = tokio::spawn(async move {
            blocker_clone.wait_for_unlock().await;
        });

        // Give it a moment to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // The wait should still be pending
        assert!(!wait_handle.is_finished());

        // Drop first guard, counter should be 1, wait should still be pending
        drop(guard1);
        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!wait_handle.is_finished());

        // Drop second guard, counter should be 0, wait should complete
        drop(guard2);

        let result = timeout(Duration::from_millis(100), wait_handle).await;
        assert!(
            result.is_ok(),
            "wait_for_unlock should complete after all guards are dropped"
        );
    }

    #[tokio::test]
    async fn test_multiple_waiters() {
        let blocker = AsyncBlocker::new();
        let guard = blocker.block_tasks();

        // Spawn multiple tasks that wait for unlock
        let blocker_clone1 = blocker.clone();
        let blocker_clone2 = blocker.clone();
        let blocker_clone3 = blocker.clone();

        let wait_handle1 = tokio::spawn(async move {
            blocker_clone1.wait_for_unlock().await;
        });
        let wait_handle2 = tokio::spawn(async move {
            blocker_clone2.wait_for_unlock().await;
        });
        let wait_handle3 = tokio::spawn(async move {
            blocker_clone3.wait_for_unlock().await;
        });

        // Give them a moment to start waiting
        tokio::time::sleep(Duration::from_millis(10)).await;

        // All waits should still be pending
        assert!(!wait_handle1.is_finished());
        assert!(!wait_handle2.is_finished());
        assert!(!wait_handle3.is_finished());

        // Drop the guard, which should notify all waiters
        drop(guard);

        // All wait_for_unlock calls should complete
        let result1 = timeout(Duration::from_millis(100), wait_handle1).await;
        let result2 = timeout(Duration::from_millis(100), wait_handle2).await;
        let result3 = timeout(Duration::from_millis(100), wait_handle3).await;

        assert!(result1.is_ok(), "first waiter should complete");
        assert!(result2.is_ok(), "second waiter should complete");
        assert!(result3.is_ok(), "third waiter should complete");
    }

    #[tokio::test]
    async fn test_clone_shares_state() {
        let blocker1 = AsyncBlocker::new();
        let blocker2 = blocker1.clone();

        // Block using blocker1
        let guard = blocker1.block_tasks();
        assert_eq!(blocker1.task_count.load(Ordering::SeqCst), 1);
        assert_eq!(blocker2.task_count.load(Ordering::SeqCst), 1);

        // Wait using blocker2
        let blocker2_clone = blocker2.clone();
        let wait_handle = tokio::spawn(async move {
            blocker2_clone.wait_for_unlock().await;
        });

        tokio::time::sleep(Duration::from_millis(10)).await;
        assert!(!wait_handle.is_finished());

        // Drop guard from blocker1, should notify blocker2's wait
        drop(guard);

        let result = timeout(Duration::from_millis(100), wait_handle).await;
        assert!(result.is_ok(), "cloned blocker should share state");
    }

    #[tokio::test]
    async fn test_rapid_block_unblock() {
        let blocker = AsyncBlocker::new();

        // Rapidly block and unblock
        for _ in 0..10 {
            let guard = blocker.block_tasks();
            assert_eq!(blocker.task_count.load(Ordering::SeqCst), 1);
            drop(guard);
            assert_eq!(blocker.task_count.load(Ordering::SeqCst), 0);
        }

        // Should still be able to wait immediately
        blocker.wait_for_unlock().await;
    }
}
