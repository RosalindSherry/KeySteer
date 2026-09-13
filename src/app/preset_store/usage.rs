//! Event-driven checkpoints. The bounded mailbox never blocks the input thread.
use super::{PresetStore, codec::Usage};
use crate::support::worker::WorkerJoin;
use std::sync::mpsc::{SyncSender, TrySendError, sync_channel};
use std::time::Duration;

pub(super) fn merge(target: &mut Usage, incoming: &Usage) {
    for (mode, count) in incoming {
        let value = target.entry(mode.clone()).or_default();
        *value = (*value).max(*count);
    }
}

pub(super) struct UsageWorker {
    sender: SyncSender<Usage>,
    join: WorkerJoin,
}

impl PresetStore {
    pub(super) fn record_entry(&mut self, mode: &str, threshold: u32) {
        if mode.is_empty()
            || mode.len() > 255
            || (self.usage.len() >= 256 && !self.usage.contains_key(mode))
        {
            return;
        }
        if !self.usage.contains_key(mode) {
            self.usage.insert(mode.to_owned(), 0);
        }
        let Some(count) = self.usage.get_mut(mode) else {
            return;
        };
        if *count == u64::MAX {
            return;
        }
        *count = count.saturating_add(1);
        self.pending_entries = self.pending_entries.saturating_add(1);
        if self.pending_entries >= threshold.max(1)
            && let Err(error) = self.queue_usage()
        {
            crate::report_error!("mode-usage", "{error}");
            // Keep the in-memory totals, but don't retry a failed worker on every key-driven transition.
            self.pending_entries = 0;
            self.worker.take();
        }
    }

    fn queue_usage(&mut self) -> Result<(), String> {
        if self.file.is_none() {
            return Ok(());
        }
        if self.worker.is_none() {
            let mut store = PresetStore {
                file: self.file.clone(),
                io: self.io.clone(),
                ..Default::default()
            };
            let (sender, receiver) = sync_channel(1);
            let join = WorkerJoin::spawn(
                "workspace-usage",
                std::thread::Builder::new().name("workspace-usage".into()),
                move || {
                    while let Ok(mut usage) = receiver.recv() {
                        // Keep only the newest snapshot if a newer checkpoint is already queued.
                        while let Ok(newer) = receiver.try_recv() {
                            merge(&mut usage, &newer);
                        }
                        store.usage = usage;
                        if let Err(error) = store.write_usage() {
                            crate::report_error!("mode-usage", "{error}");
                        }
                    }
                },
            )?;
            self.worker = Some(UsageWorker { sender, join });
        }
        if let Some(worker) = &self.worker {
            match worker.sender.try_send(self.usage.clone()) {
                Ok(()) => self.pending_entries = 0,
                Err(TrySendError::Full(_)) => {} // Retry on the next entry; retain dirty state.
                Err(TrySendError::Disconnected(_)) => {
                    return Err("Workspace usage worker stopped".into());
                }
            }
        }
        Ok(())
    }

    fn write_usage(&mut self) -> Result<(), String> {
        let io = self.io.clone();
        let _guard = io.lock().map_err(|_| "Workspace writer lock poisoned")?;
        let (layouts, mut saved) = self.read_workspace()?;
        if self
            .usage
            .iter()
            .all(|(mode, count)| saved.get(mode).is_some_and(|value| value >= count))
        {
            return Ok(());
        }
        merge(&mut saved, &self.usage);
        self.usage = saved;
        self.write_layouts(&layouts)
    }

    pub(super) fn finish_usage(&mut self) -> Result<(), String> {
        if let Some(UsageWorker { sender, mut join }) = self.worker.take() {
            drop(sender);
            join.join_timeout(Duration::from_secs(2))?;
        }
        if !self.usage.is_empty() {
            self.write_usage()?;
        }
        self.pending_entries = 0;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::window_presets::RegionTemplate;

    #[test]
    fn threshold_checkpoint_and_exit_preserve_presets_without_timer() {
        let path = super::super::tests::path();
        let mut store = PresetStore::persistent(path.clone(), super::super::tests::replace);
        store.record_entry("normal", 2);
        assert!(!path.exists());
        assert!(store.worker.is_none());
        store.record_entry("window", 2);
        let UsageWorker { sender, mut join } = store.worker.take().unwrap();
        drop(sender);
        join.join_timeout(Duration::from_secs(2)).unwrap();
        assert_eq!(store.read_workspace().unwrap().1["window"], 1);
        store.record_entry("normal", 2);
        assert_eq!(store.read_workspace().unwrap().1["normal"], 1);
        let (_, presets) = store
            .save(RegionTemplate::Slot { id: 1 }, 1, "Keep layout".into())
            .unwrap();
        assert_eq!(store.read_workspace().unwrap().1["normal"], 2);
        store.record_entry("window", 2);
        store.finish_usage().unwrap();
        let loaded = PresetStore::persistent(path.clone(), super::super::tests::replace);
        assert_eq!(loaded.list().unwrap(), presets);
        assert_eq!(loaded.usage["window"], 2);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn old_queued_snapshots_never_rollback_newer_counts_or_layout_edits() {
        let path = super::super::tests::path();
        let mut store = PresetStore::persistent(path.clone(), super::super::tests::replace);
        for _ in 0..100 {
            store.record_entry("normal", 1);
        }
        let (_, presets) = store
            .save(RegionTemplate::Slot { id: 1 }, 1, "Newest".into())
            .unwrap();
        store.delete(&presets[0]).unwrap();
        store.finish_usage().unwrap();
        let (presets, usage) = store.read_workspace().unwrap();
        assert!(presets.is_empty());
        assert_eq!(usage["normal"], 100);
        std::fs::remove_file(path).unwrap();
    }

    #[test]
    fn failed_checkpoint_does_not_destroy_previous_workspace() {
        let path = super::super::tests::path();
        let mut store = PresetStore::persistent(path.clone(), super::super::tests::replace);
        store.record_entry("normal", 100);
        store.finish_usage().unwrap();
        let before = std::fs::read(&path).unwrap();
        let mut failed = PresetStore::persistent(path.clone(), super::super::tests::fail);
        failed.record_entry("normal", 100);
        assert!(failed.finish_usage().is_err());
        assert_eq!(std::fs::read(&path).unwrap(), before);
        std::fs::remove_file(path).unwrap();
    }
}
