//! Release-to-commit arbitration for overlapping configured chords.
use super::input_router::ChordContinuation;
use super::input_state::ResolvedBinding;
use super::*;

#[derive(Debug)]
pub(super) struct PendingChord {
    key: Key,
    chord: Arc<KeyChord>,
    resolved: ResolvedBinding,
    continuations: Arc<[ChordContinuation]>,
    passthrough: bool,
    forwarded_modifiers: SmallVec<[Key; 4]>,
}

impl Engine {
    fn continuation_is_available(&self, short: &KeyChord, candidate: &ChordContinuation) -> bool {
        // Reject missing modifiers before cloning any pressed keys.
        if candidate.chord.keys().iter().any(|key| {
            key.is_modifier()
                && !self
                    .input
                    .pressed
                    .iter()
                    .any(|physical| Self::keys_match(key, physical))
        }) {
            return false;
        }
        // A generic short modifier must not promise a continuation on the
        // opposite physical side (right Alt held, left Alt required).
        if short.keys().iter().any(|key| {
            !self.input.pressed.iter().any(|physical| {
                Self::keys_match(key, physical)
                    && candidate
                        .chord
                        .keys()
                        .iter()
                        .any(|long| Self::keys_match(long, physical))
            })
        }) {
            return false;
        }

        let mut completed: SmallVec<[Key; 8]> = self.input.pressed.iter().cloned().collect();
        for key in candidate.chord.keys() {
            if !completed
                .iter()
                .any(|physical| Self::keys_match(key, physical))
            {
                completed.push(Self::injected_key(key));
            }
        }
        // Reuse the real resolver for inheritance, raw alphabets, temporary
        // modes, explicit `none`, and strict modifier ownership. Only candidates
        // already linked at compile time reach this check.
        self.lookup_for_pressed(candidate.chord.activation_key(), &completed)
            .is_some_and(|resolved| {
                resolved.owner == candidate.owner
                    && Arc::ptr_eq(&resolved.binding, &candidate.binding)
            })
    }

    pub(super) fn defer_prefix_chord(&mut self, resolved: &ResolvedBinding, key: &Key) -> bool {
        if self.registry.prefixes_require_modifier
            && !self.input.pressed.iter().any(Key::is_modifier)
        {
            return false;
        }
        let Some(entry) = self
            .registry
            .table(&resolved.owner)
            .and_then(|table| table.prefix_entry(key, &resolved.binding))
        else {
            return false;
        };
        if !entry
            .continuations
            .iter()
            .any(|candidate| self.continuation_is_available(&entry.chord, candidate))
        {
            return false;
        }
        self.input.pending_chords.push(PendingChord {
            key: key.clone(),
            chord: entry.chord.clone(),
            resolved: resolved.clone(),
            continuations: entry.continuations.clone(),
            passthrough: false,
            forwarded_modifiers: SmallVec::new(),
        });
        true
    }

    pub(super) fn defer_unbound_prefix(&mut self, key: &Key) -> bool {
        let Some(entry) = self.registry.unbound_prefixes.get(key) else {
            return false;
        };
        if !entry
            .continuations
            .iter()
            .any(|candidate| self.continuation_is_available(&entry.chord, candidate))
        {
            return false;
        }
        let forwarded_modifiers = self
            .input
            .pressed
            .iter()
            .filter(|key| {
                key.is_modifier()
                    && (self.input.key_dispositions.get(key) == Some(&KeyDisposition::Forward)
                        || self.input.replayed_keys.contains(key))
            })
            .cloned()
            .collect();
        self.input.pending_chords.push(PendingChord {
            key: key.clone(),
            chord: entry.chord.clone(),
            resolved: ResolvedBinding {
                binding: entry.binding.clone(),
                owner: self.registry.active.clone(),
            },
            continuations: entry.continuations.clone(),
            passthrough: true,
            forwarded_modifiers,
        });
        true
    }

    pub(super) fn passthrough_prefix_interrupted(
        &self,
        input: &crate::api::input::InputEvent,
    ) -> bool {
        input.state == KeyState::Down
            && !input.repeat
            && self.input.pending_chords.iter().any(|pending| {
                pending.passthrough
                    && pending.key != input.key
                    && !pending.continuations.iter().any(|candidate| {
                        candidate
                            .chord
                            .keys()
                            .iter()
                            .any(|key| Self::keys_match(key, &input.key))
                            && self.continuation_is_available(&pending.chord, candidate)
                    })
            })
    }

    fn replay_prefix(
        &mut self,
        pending: PendingChord,
        backend: &mut dyn Backend,
    ) -> Result<(), String> {
        // Already-forwarded modifiers still physically held must not receive a
        // synthetic Up. Recreate only original modifiers that have since lifted.
        let mut keys: SmallVec<[Key; 8]> = pending
            .forwarded_modifiers
            .into_iter()
            .filter(|key| !self.input.pressed.contains(key))
            .collect();
        keys.push(pending.key);
        if let Err(error) = backend.send_chord(&keys) {
            self.input
                .latched
                .extend(keys.into_iter().map(InputTarget::Key));
            return Err(self.recoverable_input_error("prefix replay", error));
        }
        Ok(())
    }

    pub(super) fn flush_passthrough_prefixes(
        &mut self,
        preserve: Option<&Key>,
        backend: &mut dyn Backend,
    ) -> Result<(), String> {
        while let Some(index) = self
            .input
            .pending_chords
            .iter()
            .position(|pending| pending.passthrough && preserve != Some(&pending.key))
        {
            let pending = self.input.pending_chords.remove(index);
            self.replay_prefix(pending, backend)?;
        }
        Ok(())
    }

    pub(super) fn cancel_completed_prefixes(&mut self, resolved: &ResolvedBinding) {
        self.input.pending_chords.retain(|pending| {
            !pending.continuations.iter().any(|candidate| {
                candidate.owner == resolved.owner
                    && Arc::ptr_eq(&candidate.binding, &resolved.binding)
                    && candidate.chord.matches_pressed(&self.input.pressed)
            })
        });
    }

    pub(super) fn is_pending_chord_key(&self, key: &Key) -> bool {
        self.input
            .pending_chords
            .iter()
            .any(|pending| &pending.key == key)
    }

    pub(super) fn has_released_prefix(&self) -> bool {
        self.input.pending_chords.iter().any(|pending| {
            !pending.chord.matches_pressed(&self.input.pressed)
                || pending
                    .forwarded_modifiers
                    .iter()
                    .any(|key| !self.input.pressed.contains(key))
        })
    }

    /// The caller has already disposed the native Up. Reuse normal action
    /// execution with a complete tap, including a release for held actions.
    pub(super) fn commit_released_prefixes(
        &mut self,
        backend: &mut dyn Backend,
    ) -> Result<(), String> {
        let active = self.registry.active.clone();
        if self.input.pending_chords.iter().any(|pending| {
            pending.passthrough
                && (!pending.chord.matches_pressed(&self.input.pressed)
                    || pending
                        .forwarded_modifiers
                        .iter()
                        .any(|key| !self.input.pressed.contains(key)))
        }) {
            self.flush_passthrough_prefixes(None, backend)?;
        }
        while let Some(index) = self
            .input
            .pending_chords
            .iter()
            .position(|pending| !pending.chord.matches_pressed(&self.input.pressed))
        {
            let pending = self.input.pending_chords.remove(index);
            let mut event = crate::api::input::InputEvent {
                character: None,
                key: pending.key.clone(),
                state: KeyState::Down,
                repeat: false,
                injected: false,
                timestamp_millis: 0,
            };
            self.apply_binding(pending.resolved.clone(), &event, backend)?;
            event.state = KeyState::Up;
            self.apply_binding(pending.resolved, &event, backend)?;
            let toggle = self.input.active_default_toggles.remove(&pending.key);
            self.finish_default_toggle(toggle, backend)?;
            if self.input.active_click_indicators.release(&pending.key) {
                self.refresh_overlay(backend)?;
            }
            if self.registry.active != active {
                break;
            }
        }
        Ok(())
    }
}
